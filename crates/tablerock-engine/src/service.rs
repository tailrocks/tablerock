use std::{collections::BTreeMap, error::Error, fmt};

use tablerock_core::{
    CancelDispatch, CommandEnvelope, EventSequence, OperationEvent, OperationId, OperationIdentity,
    OperationOutcome, OperationPhase, OperationRetireError, PageIdentity, ResultPage,
    ServiceCoordinator, ServiceError, SubscriptionId, SubscriptionStart,
};

use crate::{
    AdapterError, DriverOperationEvent, DriverOperationEvents, DriverPageRequest, DriverRuntime,
    DriverRuntimeError, DriverSession, DriverSpawnError, DriverTaskExit, RuntimeCancelOutcome,
};

#[derive(Debug)]
pub enum EngineServiceError {
    CoreSubmission {
        error: ServiceError,
        shutdown_error: Option<AdapterError>,
    },
    Core(ServiceError),
    Spawn(DriverSpawnError),
    Runtime(DriverRuntimeError),
    MissingDriverOperation,
    TerminalMismatch,
}

impl fmt::Display for EngineServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::CoreSubmission { .. } => "core rejected driver submission",
            Self::Core(_) => "core rejected driver lifecycle observation",
            Self::Spawn(_) => "driver runtime rejected operation submission",
            Self::Runtime(_) => "driver runtime operation failed",
            Self::MissingDriverOperation => "driver operation is not registered",
            Self::TerminalMismatch => "driver event and task exit disagree",
        })
    }
}

impl Error for EngineServiceError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineCancelOutcome {
    pub core: tablerock_core::CancelRequestOutcome,
    pub runtime: Option<RuntimeCancelOutcome>,
}

#[derive(Debug)]
pub enum EngineServiceUpdate {
    Started,
    Page(Box<ResultPage>),
    CancelDispatched(CancelDispatch),
    Terminal(OperationOutcome),
}

struct EngineOperation {
    events: DriverOperationEvents,
    cumulative_rows: u64,
    cumulative_bytes: u64,
}

pub struct EngineService {
    core: ServiceCoordinator,
    runtime: DriverRuntime,
    operations: BTreeMap<OperationId, EngineOperation>,
}

impl EngineService {
    #[must_use]
    pub fn new(core: ServiceCoordinator, runtime: DriverRuntime) -> Self {
        Self {
            core,
            runtime,
            operations: BTreeMap::new(),
        }
    }

    pub async fn submit(
        &mut self,
        operation_id: OperationId,
        command: CommandEnvelope,
        session: Box<dyn DriverSession>,
        request: DriverPageRequest,
        page_identity: PageIdentity,
    ) -> Result<OperationIdentity, EngineServiceError> {
        let identity = match self.core.submit(operation_id, command) {
            Ok(identity) => identity,
            Err(error) => {
                let shutdown_error = session.shutdown().await.err();
                return Err(EngineServiceError::CoreSubmission {
                    error,
                    shutdown_error,
                });
            }
        };
        let events = match self
            .runtime
            .spawn(operation_id, session, request, page_identity)
            .await
        {
            Ok(events) => events,
            Err(error) => {
                self.core
                    .transition(
                        operation_id,
                        OperationPhase::Terminal(OperationOutcome::Failed),
                    )
                    .expect("new queued operation accepts failed spawn observation");
                return Err(EngineServiceError::Spawn(error));
            }
        };
        self.operations.insert(
            operation_id,
            EngineOperation {
                events,
                cumulative_rows: 0,
                cumulative_bytes: 0,
            },
        );
        Ok(identity)
    }

    pub fn cancel(
        &mut self,
        operation_id: OperationId,
    ) -> Result<EngineCancelOutcome, EngineServiceError> {
        let core = self
            .core
            .request_cancel(operation_id)
            .map_err(EngineServiceError::Core)?;
        let runtime = match core {
            tablerock_core::CancelRequestOutcome::Requested
            | tablerock_core::CancelRequestOutcome::AlreadyRequested => {
                Some(self.runtime.cancel(operation_id))
            }
            tablerock_core::CancelRequestOutcome::AlreadyTerminal(_)
            | tablerock_core::CancelRequestOutcome::UnknownOperation => None,
        };
        Ok(EngineCancelOutcome { core, runtime })
    }

    pub async fn next_update(
        &mut self,
        operation_id: OperationId,
    ) -> Result<Option<EngineServiceUpdate>, EngineServiceError> {
        let event = self
            .operations
            .get_mut(&operation_id)
            .ok_or(EngineServiceError::MissingDriverOperation)?
            .events
            .recv()
            .await;
        let Some(event) = event else {
            let joined = self.runtime.join(operation_id).await;
            self.operations
                .remove(&operation_id)
                .ok_or(EngineServiceError::MissingDriverOperation)?;
            self.transition_unknown(operation_id)?;
            return match joined {
                Ok(_) => Err(EngineServiceError::TerminalMismatch),
                Err(error) => Err(EngineServiceError::Runtime(error)),
            };
        };
        match event {
            DriverOperationEvent::Started => {
                if self.core.operation_phase(operation_id) == Some(OperationPhase::Queued) {
                    self.core
                        .transition(operation_id, OperationPhase::Running)
                        .map_err(EngineServiceError::Core)?;
                }
                Ok(Some(EngineServiceUpdate::Started))
            }
            DriverOperationEvent::Page(page) => {
                if self.core.operation_phase(operation_id) == Some(OperationPhase::Running) {
                    self.core
                        .transition(operation_id, OperationPhase::Streaming)
                        .map_err(EngineServiceError::Core)?;
                }
                let operation = self
                    .operations
                    .get_mut(&operation_id)
                    .expect("event source remains registered");
                operation.cumulative_rows = operation
                    .cumulative_rows
                    .checked_add(u64::from(page.envelope().row_count()))
                    .ok_or(EngineServiceError::TerminalMismatch)?;
                operation.cumulative_bytes = operation
                    .cumulative_bytes
                    .checked_add(page.envelope().arena_byte_len())
                    .ok_or(EngineServiceError::TerminalMismatch)?;
                self.core
                    .progress(
                        operation_id,
                        operation.cumulative_rows,
                        operation.cumulative_bytes,
                    )
                    .map_err(EngineServiceError::Core)?;
                Ok(Some(EngineServiceUpdate::Page(page)))
            }
            DriverOperationEvent::CancelDispatched(dispatch) => {
                Ok(Some(EngineServiceUpdate::CancelDispatched(dispatch)))
            }
            DriverOperationEvent::Completed => {
                self.finish(operation_id, DriverTaskExit::Completed).await
            }
            DriverOperationEvent::ClientStopped => {
                self.finish(operation_id, DriverTaskExit::ClientStopped)
                    .await
            }
            DriverOperationEvent::Failed(error) => {
                self.finish(operation_id, DriverTaskExit::Failed(error))
                    .await
            }
        }
    }

    async fn finish(
        &mut self,
        operation_id: OperationId,
        observed: DriverTaskExit,
    ) -> Result<Option<EngineServiceUpdate>, EngineServiceError> {
        let joined = self.runtime.join(operation_id).await;
        self.operations
            .remove(&operation_id)
            .ok_or(EngineServiceError::MissingDriverOperation)?;
        let joined = match joined {
            Ok(joined) => joined,
            Err(error) => {
                self.transition_unknown(operation_id)?;
                return Err(EngineServiceError::Runtime(error));
            }
        };
        if joined != observed {
            self.transition_unknown(operation_id)?;
            return Err(EngineServiceError::TerminalMismatch);
        }
        let phase = self
            .core
            .operation_phase(operation_id)
            .ok_or(EngineServiceError::TerminalMismatch)?;
        let outcome = match (phase, observed) {
            (OperationPhase::CancelRequested, DriverTaskExit::Completed) => {
                OperationOutcome::CompletedBeforeCancel
            }
            (OperationPhase::CancelRequested, DriverTaskExit::ClientStopped) => {
                OperationOutcome::ClientStopped
            }
            (_, DriverTaskExit::Completed) => OperationOutcome::Completed,
            (_, DriverTaskExit::Failed(_)) => OperationOutcome::Failed,
            (_, DriverTaskExit::ClientStopped) => return Err(EngineServiceError::TerminalMismatch),
        };
        self.core
            .transition(operation_id, OperationPhase::Terminal(outcome))
            .map_err(EngineServiceError::Core)?;
        Ok(Some(EngineServiceUpdate::Terminal(outcome)))
    }

    fn transition_unknown(&mut self, operation_id: OperationId) -> Result<(), EngineServiceError> {
        self.core
            .transition(
                operation_id,
                OperationPhase::Terminal(OperationOutcome::Unknown),
            )
            .map(|_| ())
            .map_err(EngineServiceError::Core)
    }

    #[must_use]
    pub const fn core(&self) -> &ServiceCoordinator {
        &self.core
    }

    pub fn subscribe(
        &mut self,
        operation_id: OperationId,
        subscription_id: SubscriptionId,
        last_delivered: EventSequence,
    ) -> Result<SubscriptionStart, ServiceError> {
        self.core
            .subscribe(operation_id, subscription_id, last_delivered)
    }

    pub fn pop_event(
        &mut self,
        operation_id: OperationId,
        subscription_id: SubscriptionId,
    ) -> Result<Option<OperationEvent>, ServiceError> {
        self.core.pop_event(operation_id, subscription_id)
    }

    pub fn unsubscribe(
        &mut self,
        operation_id: OperationId,
        subscription_id: SubscriptionId,
    ) -> Result<(), ServiceError> {
        self.core.unsubscribe(operation_id, subscription_id)
    }

    pub fn retire(&mut self, operation_id: OperationId) -> Result<(), OperationRetireError> {
        self.core.retire(operation_id)
    }
}
