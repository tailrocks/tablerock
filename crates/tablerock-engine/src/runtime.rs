use std::collections::{BTreeMap, VecDeque};

use tablerock_core::{CancelDispatch, OperationId, PageIdentity, ResultPage};
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
};

use crate::{AdapterError, AdapterFailureClass, DriverPageRequest, DriverSession};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverRuntimeError {
    InvalidLimits,
    CapacityExhausted,
    DuplicateOperation,
    UnknownOperation,
    TaskFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverSpawnError {
    reason: DriverRuntimeError,
    shutdown_error: Option<AdapterError>,
}

impl DriverSpawnError {
    #[must_use]
    pub const fn reason(self) -> DriverRuntimeError {
        self.reason
    }

    #[must_use]
    pub const fn shutdown_error(self) -> Option<AdapterError> {
        self.shutdown_error
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeCancelOutcome {
    UnknownOperation,
    Queued,
    AlreadyQueued,
    TaskClosed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeStopOutcome {
    UnknownOperation,
    Requested,
    AlreadyRequested,
    TaskClosed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverTaskExit {
    Completed,
    ServerConfirmedCancelled,
    ClientStopped,
    Failed(AdapterError),
}

#[derive(Debug)]
pub enum DriverOperationEvent {
    Started,
    Page(Box<ResultPage>),
    CancelDispatched(CancelDispatch),
    Completed,
    ServerConfirmedCancelled,
    ClientStopped,
    Failed(AdapterError),
}

pub struct DriverOperationEvents {
    operation_id: OperationId,
    receiver: mpsc::Receiver<DriverOperationEvent>,
}

impl DriverOperationEvents {
    #[must_use]
    pub const fn operation_id(&self) -> OperationId {
        self.operation_id
    }

    pub async fn recv(&mut self) -> Option<DriverOperationEvent> {
        self.receiver.recv().await
    }
}

struct DriverTask {
    cancel: mpsc::Sender<()>,
    stop: watch::Sender<bool>,
    join: JoinHandle<DriverTaskExit>,
}

struct OperationTaskInput {
    operation_id: OperationId,
    session: Box<dyn DriverSession>,
    request: DriverPageRequest,
    identity: PageIdentity,
    cancels: mpsc::Receiver<()>,
    stop: watch::Receiver<bool>,
    events: mpsc::Sender<DriverOperationEvent>,
    event_capacity: usize,
}

pub struct DriverRuntime {
    max_operations: usize,
    event_capacity: usize,
    tasks: BTreeMap<OperationId, DriverTask>,
}

impl DriverRuntime {
    pub fn new(max_operations: usize, event_capacity: usize) -> Result<Self, DriverRuntimeError> {
        if max_operations == 0 || event_capacity == 0 {
            return Err(DriverRuntimeError::InvalidLimits);
        }
        Ok(Self {
            max_operations,
            event_capacity,
            tasks: BTreeMap::new(),
        })
    }

    pub async fn spawn(
        &mut self,
        operation_id: OperationId,
        session: Box<dyn DriverSession>,
        request: DriverPageRequest,
        identity: PageIdentity,
    ) -> Result<DriverOperationEvents, DriverSpawnError> {
        if self.tasks.contains_key(&operation_id) {
            return Err(reject_session(session, DriverRuntimeError::DuplicateOperation).await);
        }
        if self.tasks.len() >= self.max_operations {
            return Err(reject_session(session, DriverRuntimeError::CapacityExhausted).await);
        }
        let (cancel_tx, cancel_rx) = mpsc::channel(1);
        let (stop_tx, stop_rx) = watch::channel(false);
        let (event_tx, event_rx) = mpsc::channel(self.event_capacity);
        let event_capacity = self.event_capacity;
        let join = tokio::spawn(run_operation(OperationTaskInput {
            operation_id,
            session,
            request,
            identity,
            cancels: cancel_rx,
            stop: stop_rx,
            events: event_tx,
            event_capacity,
        }));
        self.tasks.insert(
            operation_id,
            DriverTask {
                cancel: cancel_tx,
                stop: stop_tx,
                join,
            },
        );
        Ok(DriverOperationEvents {
            operation_id,
            receiver: event_rx,
        })
    }

    pub fn cancel(&self, operation_id: OperationId) -> RuntimeCancelOutcome {
        let Some(task) = self.tasks.get(&operation_id) else {
            return RuntimeCancelOutcome::UnknownOperation;
        };
        match task.cancel.try_send(()) {
            Ok(()) => RuntimeCancelOutcome::Queued,
            Err(mpsc::error::TrySendError::Full(())) => RuntimeCancelOutcome::AlreadyQueued,
            Err(mpsc::error::TrySendError::Closed(())) => RuntimeCancelOutcome::TaskClosed,
        }
    }

    pub fn stop_client(&self, operation_id: OperationId) -> RuntimeStopOutcome {
        let Some(task) = self.tasks.get(&operation_id) else {
            return RuntimeStopOutcome::UnknownOperation;
        };
        if *task.stop.borrow() {
            return RuntimeStopOutcome::AlreadyRequested;
        }
        match task.stop.send(true) {
            Ok(()) => RuntimeStopOutcome::Requested,
            Err(_) => RuntimeStopOutcome::TaskClosed,
        }
    }

    pub async fn join(
        &mut self,
        operation_id: OperationId,
    ) -> Result<DriverTaskExit, DriverRuntimeError> {
        let task = self
            .tasks
            .remove(&operation_id)
            .ok_or(DriverRuntimeError::UnknownOperation)?;
        let DriverTask { cancel, stop, join } = task;
        let result = join.await.map_err(|_| DriverRuntimeError::TaskFailed);
        drop((cancel, stop));
        result
    }

    pub async fn shutdown(self) -> Result<(), DriverRuntimeError> {
        for task in self.tasks.values() {
            task.stop.send_replace(true);
        }
        let mut failed = false;
        for (_, task) in self.tasks {
            match task.join.await {
                Ok(
                    DriverTaskExit::Completed
                    | DriverTaskExit::ServerConfirmedCancelled
                    | DriverTaskExit::ClientStopped,
                ) => {}
                Ok(DriverTaskExit::Failed(_)) | Err(_) => failed = true,
            }
        }
        if failed {
            return Err(DriverRuntimeError::TaskFailed);
        }
        Ok(())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

async fn reject_session(
    session: Box<dyn DriverSession>,
    reason: DriverRuntimeError,
) -> DriverSpawnError {
    DriverSpawnError {
        reason,
        shutdown_error: session.shutdown().await.err(),
    }
}

async fn run_operation(input: OperationTaskInput) -> DriverTaskExit {
    let OperationTaskInput {
        operation_id,
        session,
        request,
        identity,
        mut cancels,
        mut stop,
        events,
        event_capacity,
    } = input;
    let mut pending = VecDeque::with_capacity(event_capacity);
    let stream_start = session.start_page_stream(request);
    pending.push_back(DriverOperationEvent::Started);
    let mut cancel_dispatched = false;
    let stream_result = StreamStartControl {
        operation_id,
        session: session.as_ref(),
        cancels: &mut cancels,
        stop: &mut stop,
        events: &events,
        pending: &mut pending,
        cancel_dispatched: &mut cancel_dispatched,
    }
    .start(stream_start)
    .await;
    let Some(stream_result) = stream_result else {
        let _ = events.try_send(DriverOperationEvent::ClientStopped);
        let _ = session.shutdown().await;
        return DriverTaskExit::ClientStopped;
    };
    let mut stream = match stream_result {
        Ok(stream) => stream,
        Err(error) => {
            while let Some(event) = pending.pop_front() {
                if events.send(event).await.is_err() {
                    break;
                }
            }
            let (event, exit) = terminal_event(error);
            let _ = events.send(event).await;
            let _ = session.shutdown().await;
            return exit;
        }
    };
    let mut start_row = 0_u64;
    let mut stop_client = false;
    let mut terminal_error = None;

    loop {
        if let Some(event) = pending.pop_front() {
            tokio::select! {
                permit = events.reserve() => {
                    let Ok(permit) = permit else { break; };
                    permit.send(event);
                }
                changed = stop.changed() => {
                    pending.push_front(event);
                    if changed.is_err() || *stop.borrow_and_update() {
                        stop_client = true;
                        break;
                    }
                }
                cancel = cancels.recv() => {
                    pending.push_front(event);
                    if cancel.is_some() {
                        handle_cancel(
                            operation_id,
                            session.as_ref(),
                            &mut pending,
                            &mut cancel_dispatched,
                        ).await;
                    }
                }
            }
            continue;
        }

        tokio::select! {
            changed = stop.changed() => {
                if changed.is_err() || *stop.borrow_and_update() {
                    stop_client = true;
                    break;
                }
            }
            cancel = cancels.recv() => {
                if cancel.is_some() {
                    handle_cancel(
                        operation_id,
                        session.as_ref(),
                        &mut pending,
                        &mut cancel_dispatched,
                    ).await;
                }
            }
            page = stream.next_page(identity, start_row) => {
                match page {
                    Ok(Some(page)) => {
                        let Some(next_start) = start_row
                            .checked_add(u64::from(page.envelope().row_count()))
                        else {
                            terminal_error = Some(AdapterError::new(
                                identity.engine(),
                                AdapterFailureClass::Page,
                            ));
                            break;
                        };
                        start_row = next_start;
                        pending.push_back(DriverOperationEvent::Page(Box::new(page)));
                    }
                    Ok(None) => {
                        break;
                    }
                    Err(error) => {
                        terminal_error = Some(error);
                        break;
                    }
                }
            }
        }
    }

    if stop_client {
        let _ = events.try_send(DriverOperationEvent::ClientStopped);
        drop(stream);
        let _ = session.shutdown().await;
        return DriverTaskExit::ClientStopped;
    }
    while let Some(event) = pending.pop_front() {
        if events.send(event).await.is_err() {
            break;
        }
    }
    drop(stream);
    if let Err(error) = session.shutdown().await {
        terminal_error = Some(error);
    }
    let (event, exit) = match terminal_error {
        Some(error) => terminal_event(error),
        None => (DriverOperationEvent::Completed, DriverTaskExit::Completed),
    };
    let _ = events.send(event).await;
    exit
}

struct StreamStartControl<'a> {
    operation_id: OperationId,
    session: &'a dyn DriverSession,
    cancels: &'a mut mpsc::Receiver<()>,
    stop: &'a mut watch::Receiver<bool>,
    events: &'a mpsc::Sender<DriverOperationEvent>,
    pending: &'a mut VecDeque<DriverOperationEvent>,
    cancel_dispatched: &'a mut bool,
}

impl StreamStartControl<'_> {
    async fn start(
        &mut self,
        mut start: crate::DriverFuture<'_, Result<Box<dyn crate::DriverPageStream>, AdapterError>>,
    ) -> Option<Result<Box<dyn crate::DriverPageStream>, AdapterError>> {
        loop {
            tokio::select! {
                biased;
                result = &mut start => return Some(result),
                permit = self.events.reserve(), if !self.pending.is_empty() => {
                    let Ok(permit) = permit else { return None; };
                    permit.send(self.pending.pop_front().expect("guarded pending event"));
                }
                cancel = self.cancels.recv() => {
                    if cancel.is_some() {
                        handle_cancel(
                            self.operation_id,
                            self.session,
                            self.pending,
                            self.cancel_dispatched,
                        ).await;
                    }
                }
                changed = self.stop.changed() => {
                    if changed.is_err() || *self.stop.borrow_and_update() {
                        return None;
                    }
                }
            }
        }
    }
}

fn terminal_event(error: AdapterError) -> (DriverOperationEvent, DriverTaskExit) {
    if error.class() == AdapterFailureClass::ServerCancelled {
        (
            DriverOperationEvent::ServerConfirmedCancelled,
            DriverTaskExit::ServerConfirmedCancelled,
        )
    } else {
        (
            DriverOperationEvent::Failed(error),
            DriverTaskExit::Failed(error),
        )
    }
}

async fn handle_cancel(
    operation_id: OperationId,
    session: &dyn DriverSession,
    pending: &mut VecDeque<DriverOperationEvent>,
    cancel_dispatched: &mut bool,
) {
    if !*cancel_dispatched {
        let dispatch = session.cancel(operation_id).await;
        pending.push_back(DriverOperationEvent::CancelDispatched(dispatch));
        *cancel_dispatched = true;
    }
}
