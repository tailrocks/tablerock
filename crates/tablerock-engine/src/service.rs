use std::{collections::BTreeMap, error::Error, fmt, sync::Arc};

use tablerock_core::{
    CancelDispatch, CatalogCursor, CatalogIdentity, CatalogLimits, CatalogNode, CatalogNodeId,
    CatalogSnapshot, CommandEnvelope, CommandIntent, CommandScope, EventSequence, IdParts,
    OperationEvent, OperationId, OperationIdentity, OperationOutcome, OperationPhase,
    OperationRetireError, PageIdentity, ResultPage, ServiceCoordinator, ServiceError, ServicePhase,
    SessionId, ShutdownMode, ShutdownOutcome, SubscriptionId, SubscriptionStart,
};

use crate::{
    AdapterError, CatalogRequest, CatalogSubtree, DriverOperationEvent, DriverOperationEvents,
    DriverPageRequest, DriverRuntime, DriverRuntimeError, DriverSession, DriverSpawnError,
    DriverTaskExit, RuntimeCancelOutcome, RuntimeStopOutcome, SessionRegistry,
    SessionRegistryError,
};

#[derive(Debug)]
pub enum EngineServiceError {
    CoreSubmission { error: ServiceError },
    Core(ServiceError),
    Spawn(DriverSpawnError),
    Runtime(DriverRuntimeError),
    Session(SessionRegistryError),
    SessionBusy,
    MissingDriverOperation,
    TerminalMismatch,
    ShutdownStillDraining,
    RuntimeUnavailable,
    IntentMismatch,
    Catalog(AdapterError),
    CatalogBuild(tablerock_core::CatalogBuildError),
    CatalogCursor(tablerock_core::CatalogRejection),
}

impl fmt::Display for EngineServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::CoreSubmission { .. } => "core rejected driver submission",
            Self::Core(_) => "core rejected driver lifecycle observation",
            Self::Spawn(_) => "driver runtime rejected operation submission",
            Self::Runtime(_) => "driver runtime operation failed",
            Self::Session(_) => "session registry rejected the request",
            Self::SessionBusy => "session still has active operation borrows",
            Self::MissingDriverOperation => "driver operation is not registered",
            Self::TerminalMismatch => "driver event and task exit disagree",
            Self::ShutdownStillDraining => "engine service shutdown is still draining",
            Self::RuntimeUnavailable => "engine service runtime is unavailable",
            Self::IntentMismatch => "command intent is not a catalog refresh",
            Self::Catalog(_) => "catalog listing failed",
            Self::CatalogBuild(error) => return write!(formatter, "{error}"),
            Self::CatalogCursor(_) => "catalog cursor rejected the snapshot",
        })
    }
}

impl Error for EngineServiceError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineCancelOutcome {
    pub core: tablerock_core::CancelRequestOutcome,
    pub runtime: Option<RuntimeCancelOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineShutdownOutcome {
    pub core: ShutdownOutcome,
    pub client_stops: Box<[(OperationId, RuntimeStopOutcome)]>,
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
    runtime: Option<DriverRuntime>,
    sessions: SessionRegistry,
    operations: BTreeMap<OperationId, EngineOperation>,
}

impl EngineService {
    pub fn new(
        core: ServiceCoordinator,
        runtime: DriverRuntime,
        max_sessions: usize,
    ) -> Result<Self, EngineServiceError> {
        Ok(Self {
            core,
            runtime: Some(runtime),
            sessions: SessionRegistry::new(max_sessions).map_err(EngineServiceError::Session)?,
            operations: BTreeMap::new(),
        })
    }

    pub fn register_session(
        &mut self,
        session_id: SessionId,
        session: Box<dyn DriverSession>,
    ) -> Result<Arc<dyn DriverSession>, EngineServiceError> {
        self.sessions
            .register(session_id, session)
            .map_err(EngineServiceError::Session)
    }

    #[must_use]
    pub fn session(&self, session_id: SessionId) -> Option<Arc<dyn DriverSession>> {
        self.sessions.session(session_id)
    }

    pub async fn disconnect(&mut self, session_id: SessionId) -> Result<(), EngineServiceError> {
        match self.sessions.disconnect(session_id).await {
            Ok(()) => Ok(()),
            Err(SessionRegistryError::SessionBusy) => Err(EngineServiceError::SessionBusy),
            Err(error) => Err(EngineServiceError::Session(error)),
        }
    }

    /// List one catalog level, assemble a validated snapshot, and advance scope revision.
    #[allow(clippy::too_many_arguments)]
    pub async fn refresh_catalog(
        &mut self,
        session: Arc<dyn DriverSession>,
        command: CommandEnvelope,
        request: CatalogRequest,
        cursor: CatalogCursor,
        limits: CatalogLimits,
        id_seed: u64,
        parent: Option<(CatalogNodeId, u16)>,
    ) -> Result<(CatalogSnapshot, CatalogCursor), EngineServiceError> {
        if !matches!(command.intent(), CommandIntent::RefreshCatalog) {
            return Err(EngineServiceError::IntentMismatch);
        }
        let CommandScope::Context(scope) = command.scope() else {
            return Err(EngineServiceError::IntentMismatch);
        };
        let expected = command.expected_revision();
        let subtree = session
            .catalog(request)
            .await
            .map_err(EngineServiceError::Catalog)?;
        if subtree.engine() != session.engine() {
            return Err(EngineServiceError::Catalog(AdapterError::new(
                session.engine(),
                crate::AdapterFailureClass::EngineMismatch,
            )));
        }
        let next_revision = self
            .core
            .advance_scope(CommandScope::Context(scope), expected)
            .map_err(EngineServiceError::Core)?;
        let identity = CatalogIdentity::new(scope, subtree.engine(), next_revision);
        let nodes = assemble_catalog_nodes(subtree, parent, id_seed);
        let snapshot = CatalogSnapshot::new(identity, nodes, limits)
            .map_err(EngineServiceError::CatalogBuild)?;
        let next_cursor = cursor
            .accept(&snapshot)
            .map_err(EngineServiceError::CatalogCursor)?;
        Ok((snapshot, next_cursor))
    }

    pub async fn submit(
        &mut self,
        operation_id: OperationId,
        command: CommandEnvelope,
        session: Arc<dyn DriverSession>,
        request: DriverPageRequest,
        page_identity: PageIdentity,
    ) -> Result<OperationIdentity, EngineServiceError> {
        let identity = match self.core.submit(operation_id, command) {
            Ok(identity) => identity,
            Err(error) => {
                // Drop the Arc borrow only; the registry keeps the connection.
                drop(session);
                return Err(EngineServiceError::CoreSubmission { error });
            }
        };
        let events = match self
            .runtime
            .as_mut()
            .ok_or(EngineServiceError::RuntimeUnavailable)?
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
            | tablerock_core::CancelRequestOutcome::AlreadyRequested => Some(
                self.runtime
                    .as_ref()
                    .ok_or(EngineServiceError::RuntimeUnavailable)?
                    .cancel(operation_id),
            ),
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
            let joined = self
                .runtime
                .as_mut()
                .ok_or(EngineServiceError::RuntimeUnavailable)?
                .join(operation_id)
                .await;
            self.operations
                .remove(&operation_id)
                .ok_or(EngineServiceError::MissingDriverOperation)?;
            return match joined {
                Ok(observed) => self.apply_terminal(operation_id, observed),
                Err(error) => {
                    self.transition_unknown(operation_id)?;
                    Err(EngineServiceError::Runtime(error))
                }
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
            DriverOperationEvent::ServerConfirmedCancelled => {
                self.finish(operation_id, DriverTaskExit::ServerConfirmedCancelled)
                    .await
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
        let joined = self
            .runtime
            .as_mut()
            .ok_or(EngineServiceError::RuntimeUnavailable)?
            .join(operation_id)
            .await;
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
        self.apply_terminal(operation_id, observed)
    }

    fn apply_terminal(
        &mut self,
        operation_id: OperationId,
        observed: DriverTaskExit,
    ) -> Result<Option<EngineServiceUpdate>, EngineServiceError> {
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
            (OperationPhase::CancelRequested, DriverTaskExit::ServerConfirmedCancelled) => {
                OperationOutcome::ServerConfirmedCancelled
            }
            (_, DriverTaskExit::Completed) => OperationOutcome::Completed,
            (_, DriverTaskExit::ServerConfirmedCancelled) => OperationOutcome::Failed,
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

    /// Mutable coordinator access for scope registration and event fan-out.
    pub fn core_mut(&mut self) -> &mut ServiceCoordinator {
        &mut self.core
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

    pub fn begin_shutdown(
        &mut self,
        mode: ShutdownMode,
    ) -> Result<EngineShutdownOutcome, EngineServiceError> {
        let core = self
            .core
            .begin_shutdown(mode)
            .map_err(EngineServiceError::Core)?;
        let mut client_stops = Vec::new();
        if mode == ShutdownMode::CancelActive {
            let runtime = self
                .runtime
                .as_ref()
                .ok_or(EngineServiceError::RuntimeUnavailable)?;
            for operation_id in self.operations.keys().copied() {
                if self.core.operation_phase(operation_id) == Some(OperationPhase::CancelRequested)
                {
                    client_stops.push((operation_id, runtime.stop_client(operation_id)));
                }
            }
        }
        Ok(EngineShutdownOutcome {
            core,
            client_stops: client_stops.into_boxed_slice(),
        })
    }

    pub async fn complete_shutdown(&mut self) -> Result<(), EngineServiceError> {
        if self.core.phase() != ServicePhase::Stopped || !self.operations.is_empty() {
            return Err(EngineServiceError::ShutdownStillDraining);
        }
        let runtime = self
            .runtime
            .take()
            .ok_or(EngineServiceError::RuntimeUnavailable)?;
        runtime
            .shutdown()
            .await
            .map_err(EngineServiceError::Runtime)
    }
}

fn assemble_catalog_nodes(
    subtree: CatalogSubtree,
    parent: Option<(CatalogNodeId, u16)>,
    id_seed: u64,
) -> Vec<CatalogNode> {
    let (parent_id, parent_depth) = match parent {
        Some((id, depth)) => (Some(id), depth),
        None => (None, 0),
    };
    let depth = if parent_id.is_some() {
        parent_depth.saturating_add(1)
    } else {
        0
    };
    subtree
        .into_nodes()
        .into_iter()
        .enumerate()
        .filter_map(|(index, seed)| {
            let id = CatalogNodeId::from_parts(
                IdParts::new(0, id_seed.saturating_add(index as u64 + 1)).ok()?,
            )
            .ok()?;
            let name = seed.clone().into_name();
            let engine_type = seed.clone().take_engine_type();
            Some(CatalogNode::new(
                id,
                parent_id,
                depth,
                seed.kind(),
                name,
                engine_type,
                seed.children(),
            ))
        })
        .collect()
}
