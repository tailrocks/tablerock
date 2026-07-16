use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tablerock_core::{Engine, IdParts, OperationId, PageIdentity, ResultId, Revision};
use tablerock_engine::{
    AdapterError, CancelDispatch, DriverFuture, DriverOperationEvent, DriverPageRequest,
    DriverPageStream, DriverRuntime, DriverRuntimeError, DriverSession, DriverTaskExit,
    PostgresProbeQuery, RuntimeCancelOutcome,
};
use tokio::sync::Notify;
use tokio::time::{Duration, timeout};

struct WaitingStream {
    release: Arc<Notify>,
    released: Arc<AtomicBool>,
    started: Arc<AtomicBool>,
}

impl DriverPageStream for WaitingStream {
    fn next_page<'a>(
        &'a mut self,
        _identity: PageIdentity,
        _start_row: u64,
    ) -> DriverFuture<'a, Result<Option<tablerock_core::ResultPage>, AdapterError>> {
        self.started.store(true, Ordering::SeqCst);
        Box::pin(async move {
            if !self.released.load(Ordering::SeqCst) {
                self.release.notified().await;
            }
            Ok(None)
        })
    }
}

struct ControlledSession {
    release: Arc<Notify>,
    released: Arc<AtomicBool>,
    started: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
}

impl DriverSession for ControlledSession {
    fn engine(&self) -> Engine {
        Engine::PostgreSql
    }

    fn start_page_stream<'a>(
        &'a self,
        _request: DriverPageRequest,
    ) -> DriverFuture<'a, Result<Box<dyn DriverPageStream>, AdapterError>> {
        let release = Arc::clone(&self.release);
        let released = Arc::clone(&self.released);
        Box::pin(async move {
            Ok(Box::new(WaitingStream {
                release,
                released,
                started: Arc::clone(&self.started),
            }) as Box<dyn DriverPageStream>)
        })
    }

    fn cancel<'a>(&'a self, _operation_id: OperationId) -> DriverFuture<'a, CancelDispatch> {
        Box::pin(async move {
            self.released.store(true, Ordering::SeqCst);
            self.release.notify_one();
            CancelDispatch::RequestSent
        })
    }

    fn shutdown(self: Box<Self>) -> DriverFuture<'static, Result<(), AdapterError>> {
        Box::pin(async move {
            self.shutdown.store(true, Ordering::SeqCst);
            Ok(())
        })
    }
}

fn operation(value: u64) -> OperationId {
    OperationId::from_parts(IdParts::new(0, value).unwrap()).unwrap()
}

fn identity() -> PageIdentity {
    PageIdentity::new(
        ResultId::from_parts(IdParts::new(0, 1).unwrap()).unwrap(),
        Revision::INITIAL,
        Engine::PostgreSql,
    )
}

fn request() -> DriverPageRequest {
    DriverPageRequest::PostgreSqlProbe {
        query: PostgresProbeQuery::BoundedSeries,
        limits: tablerock_core::PageLimits::new(1, 1, 1, 1),
        max_cell_bytes: 1,
    }
}

fn session(shutdown: Arc<AtomicBool>) -> (Box<dyn DriverSession>, Arc<AtomicBool>) {
    let started = Arc::new(AtomicBool::new(false));
    (
        Box::new(ControlledSession {
            release: Arc::new(Notify::new()),
            released: Arc::new(AtomicBool::new(false)),
            started: Arc::clone(&started),
            shutdown,
        }),
        started,
    )
}

#[tokio::test]
async fn routes_cancel_while_output_is_backpressured() {
    let operation_id = operation(1);
    let shutdown = Arc::new(AtomicBool::new(false));
    let (session, started) = session(Arc::clone(&shutdown));
    let mut runtime = DriverRuntime::new(1, 1).unwrap();
    let mut events = runtime
        .spawn(operation_id, session, request(), identity())
        .unwrap();

    while !started.load(Ordering::SeqCst) {
        tokio::task::yield_now().await;
    }
    assert_eq!(runtime.cancel(operation_id), RuntimeCancelOutcome::Queued);
    assert!(matches!(
        timeout(Duration::from_secs(1), events.recv())
            .await
            .expect("started event remains responsive"),
        Some(DriverOperationEvent::Started)
    ));
    assert!(matches!(
        timeout(Duration::from_secs(1), events.recv())
            .await
            .expect("cancel dispatch remains responsive"),
        Some(DriverOperationEvent::CancelDispatched(
            CancelDispatch::RequestSent
        ))
    ));
    assert!(matches!(
        timeout(Duration::from_secs(1), events.recv())
            .await
            .expect("completion remains responsive"),
        Some(DriverOperationEvent::Completed)
    ));
    assert_eq!(
        timeout(Duration::from_secs(1), runtime.join(operation_id))
            .await
            .expect("task join remains responsive")
            .unwrap(),
        DriverTaskExit::Completed
    );
    assert!(shutdown.load(Ordering::SeqCst));
}

#[tokio::test]
async fn bounds_tasks_and_stops_without_waiting_for_slow_event_consumers() {
    let first = operation(1);
    let second = operation(2);
    let mut runtime = DriverRuntime::new(1, 1).unwrap();
    let (first_session, _) = session(Arc::new(AtomicBool::new(false)));
    let _events = runtime
        .spawn(first, first_session, request(), identity())
        .unwrap();
    let (duplicate_session, _) = session(Arc::new(AtomicBool::new(false)));
    assert!(matches!(
        runtime.spawn(first, duplicate_session, request(), identity()),
        Err(DriverRuntimeError::DuplicateOperation)
    ));
    let (overflow_session, _) = session(Arc::new(AtomicBool::new(false)));
    assert!(matches!(
        runtime.spawn(second, overflow_session, request(), identity()),
        Err(DriverRuntimeError::CapacityExhausted)
    ));
    assert_eq!(
        runtime.cancel(second),
        RuntimeCancelOutcome::UnknownOperation
    );
    runtime.shutdown().await.unwrap();
}
