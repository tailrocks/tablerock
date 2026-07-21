use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tablerock_core::{
    CancelDispatch, Engine, IdParts, OperationId, PageIdentity, ResultId, Revision,
};
use tablerock_engine::{
    AdapterError, AdapterFailureClass, DriverFuture, DriverOperationEvent, DriverPageRequest,
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
    fail_shutdown: bool,
}

struct StartingSession {
    release: Arc<Notify>,
    cancelled: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
}

struct CoupledStream {
    cancel_requested: Arc<Notify>,
    stream_drained: Arc<Notify>,
}

impl DriverPageStream for CoupledStream {
    fn next_page<'a>(
        &'a mut self,
        _identity: PageIdentity,
        _start_row: u64,
    ) -> DriverFuture<'a, Result<Option<tablerock_core::ResultPage>, AdapterError>> {
        Box::pin(async move {
            self.cancel_requested.notified().await;
            self.stream_drained.notify_one();
            Ok(None)
        })
    }
}

struct CoupledCancelSession {
    cancel_requested: Arc<Notify>,
    stream_drained: Arc<Notify>,
}

impl DriverSession for CoupledCancelSession {
    fn engine(&self) -> Engine {
        Engine::ClickHouse
    }

    fn start_page_stream<'a>(
        &'a self,
        _request: DriverPageRequest,
    ) -> DriverFuture<'a, Result<Box<dyn DriverPageStream>, AdapterError>> {
        let cancel_requested = Arc::clone(&self.cancel_requested);
        let stream_drained = Arc::clone(&self.stream_drained);
        Box::pin(async move {
            Ok(Box::new(CoupledStream {
                cancel_requested,
                stream_drained,
            }) as Box<dyn DriverPageStream>)
        })
    }

    fn cancel_requires_stream_progress(&self) -> bool {
        true
    }

    fn cancel<'a>(&'a self, _operation_id: OperationId) -> DriverFuture<'a, CancelDispatch> {
        Box::pin(async move {
            self.cancel_requested.notify_one();
            self.stream_drained.notified().await;
            CancelDispatch::RequestSent
        })
    }

    fn health<'a>(
        &'a self,
    ) -> DriverFuture<'a, Result<tablerock_engine::SessionHealth, AdapterError>> {
        Box::pin(async {
            Ok(tablerock_engine::SessionHealth::new(
                Engine::ClickHouse,
                true,
                0,
            ))
        })
    }

    fn catalog<'a>(
        &'a self,
        _request: tablerock_engine::CatalogRequest,
    ) -> DriverFuture<'a, Result<tablerock_engine::CatalogSubtree, AdapterError>> {
        Box::pin(async {
            Err(AdapterError::new(
                Engine::ClickHouse,
                AdapterFailureClass::InvalidRequest,
            ))
        })
    }

    fn describe<'a>(
        &'a self,
    ) -> DriverFuture<'a, Result<tablerock_engine::ServerDescribe, AdapterError>> {
        Box::pin(async {
            Ok(tablerock_engine::ServerDescribe::new(
                Engine::ClickHouse,
                "test",
                0,
            ))
        })
    }

    fn shutdown(self: Box<Self>) -> DriverFuture<'static, Result<(), AdapterError>> {
        Box::pin(async { Ok(()) })
    }
}

impl DriverSession for StartingSession {
    fn engine(&self) -> Engine {
        Engine::PostgreSql
    }

    fn start_page_stream<'a>(
        &'a self,
        _request: DriverPageRequest,
    ) -> DriverFuture<'a, Result<Box<dyn DriverPageStream>, AdapterError>> {
        Box::pin(async move {
            if !self.cancelled.load(Ordering::SeqCst) {
                self.release.notified().await;
            }
            Err(AdapterError::new(
                Engine::PostgreSql,
                AdapterFailureClass::ServerCancelled,
            ))
        })
    }

    fn cancel<'a>(&'a self, _operation_id: OperationId) -> DriverFuture<'a, CancelDispatch> {
        Box::pin(async move {
            self.cancelled.store(true, Ordering::SeqCst);
            self.release.notify_one();
            CancelDispatch::RequestSent
        })
    }

    fn health<'a>(
        &'a self,
    ) -> DriverFuture<'a, Result<tablerock_engine::SessionHealth, AdapterError>> {
        Box::pin(async {
            Ok(tablerock_engine::SessionHealth::new(
                Engine::PostgreSql,
                true,
                0,
            ))
        })
    }

    fn catalog<'a>(
        &'a self,
        _request: tablerock_engine::CatalogRequest,
    ) -> DriverFuture<'a, Result<tablerock_engine::CatalogSubtree, AdapterError>> {
        Box::pin(async {
            Err(AdapterError::new(
                Engine::PostgreSql,
                tablerock_engine::AdapterFailureClass::InvalidRequest,
            ))
        })
    }

    fn describe<'a>(
        &'a self,
    ) -> DriverFuture<'a, Result<tablerock_engine::ServerDescribe, AdapterError>> {
        Box::pin(async {
            Ok(tablerock_engine::ServerDescribe::new(
                Engine::PostgreSql,
                "test",
                0,
            ))
        })
    }

    fn shutdown(self: Box<Self>) -> DriverFuture<'static, Result<(), AdapterError>> {
        Box::pin(async move {
            self.shutdown.store(true, Ordering::SeqCst);
            Ok(())
        })
    }
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

    fn health<'a>(
        &'a self,
    ) -> DriverFuture<'a, Result<tablerock_engine::SessionHealth, AdapterError>> {
        Box::pin(async {
            Ok(tablerock_engine::SessionHealth::new(
                Engine::PostgreSql,
                true,
                0,
            ))
        })
    }

    fn catalog<'a>(
        &'a self,
        _request: tablerock_engine::CatalogRequest,
    ) -> DriverFuture<'a, Result<tablerock_engine::CatalogSubtree, AdapterError>> {
        Box::pin(async {
            Err(AdapterError::new(
                Engine::PostgreSql,
                tablerock_engine::AdapterFailureClass::InvalidRequest,
            ))
        })
    }

    fn describe<'a>(
        &'a self,
    ) -> DriverFuture<'a, Result<tablerock_engine::ServerDescribe, AdapterError>> {
        Box::pin(async {
            Ok(tablerock_engine::ServerDescribe::new(
                Engine::PostgreSql,
                "test",
                0,
            ))
        })
    }

    fn shutdown(self: Box<Self>) -> DriverFuture<'static, Result<(), AdapterError>> {
        Box::pin(async move {
            self.shutdown.store(true, Ordering::SeqCst);
            if self.fail_shutdown {
                Err(AdapterError::new(
                    Engine::PostgreSql,
                    AdapterFailureClass::Connection,
                ))
            } else {
                Ok(())
            }
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

fn session(shutdown: Arc<AtomicBool>) -> (Arc<dyn DriverSession>, Arc<AtomicBool>) {
    let started = Arc::new(AtomicBool::new(false));
    (
        Arc::new(ControlledSession {
            release: Arc::new(Notify::new()),
            released: Arc::new(AtomicBool::new(false)),
            started: Arc::clone(&started),
            shutdown,
            fail_shutdown: false,
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
        .await
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
    // Runtime no longer shuts the session down at terminal.
    assert!(!shutdown.load(Ordering::SeqCst));
}

#[tokio::test]
async fn drains_stream_while_cancel_transport_waits_for_it() {
    let operation_id = operation(6);
    let mut runtime = DriverRuntime::new(1, 2).unwrap();
    let mut events = runtime
        .spawn(
            operation_id,
            Arc::new(CoupledCancelSession {
                cancel_requested: Arc::new(Notify::new()),
                stream_drained: Arc::new(Notify::new()),
            }),
            request(),
            identity(),
        )
        .await
        .unwrap();

    assert!(matches!(
        timeout(Duration::from_secs(1), events.recv())
            .await
            .unwrap(),
        Some(DriverOperationEvent::Started)
    ));
    assert_eq!(runtime.cancel(operation_id), RuntimeCancelOutcome::Queued);
    assert!(matches!(
        timeout(Duration::from_secs(1), events.recv())
            .await
            .unwrap(),
        Some(DriverOperationEvent::CancelDispatched(
            CancelDispatch::RequestSent
        ))
    ));
    assert!(matches!(
        timeout(Duration::from_secs(1), events.recv())
            .await
            .unwrap(),
        Some(DriverOperationEvent::Completed)
    ));
    assert_eq!(
        timeout(Duration::from_secs(1), runtime.join(operation_id))
            .await
            .unwrap()
            .unwrap(),
        DriverTaskExit::Completed
    );
}

#[tokio::test]
async fn routes_cancel_while_stream_is_starting() {
    let operation_id = operation(7);
    let shutdown = Arc::new(AtomicBool::new(false));
    let mut runtime = DriverRuntime::new(1, 2).unwrap();
    let mut events = runtime
        .spawn(
            operation_id,
            Arc::new(StartingSession {
                release: Arc::new(Notify::new()),
                cancelled: Arc::new(AtomicBool::new(false)),
                shutdown: Arc::clone(&shutdown),
            }),
            request(),
            identity(),
        )
        .await
        .unwrap();

    assert!(matches!(
        timeout(Duration::from_secs(1), events.recv())
            .await
            .unwrap(),
        Some(DriverOperationEvent::Started)
    ));
    assert_eq!(runtime.cancel(operation_id), RuntimeCancelOutcome::Queued);
    assert!(matches!(
        timeout(Duration::from_secs(1), events.recv())
            .await
            .unwrap(),
        Some(DriverOperationEvent::CancelDispatched(
            CancelDispatch::RequestSent
        ))
    ));
    assert!(matches!(
        timeout(Duration::from_secs(1), events.recv())
            .await
            .unwrap(),
        Some(DriverOperationEvent::ServerConfirmedCancelled)
    ));
    assert_eq!(
        timeout(Duration::from_secs(1), runtime.join(operation_id))
            .await
            .unwrap()
            .unwrap(),
        DriverTaskExit::ServerConfirmedCancelled
    );
    assert!(!shutdown.load(Ordering::SeqCst));
}

#[tokio::test]
async fn bounds_tasks_and_stops_without_waiting_for_slow_event_consumers() {
    let first = operation(1);
    let second = operation(2);
    let mut runtime = DriverRuntime::new(1, 1).unwrap();
    let (first_session, _) = session(Arc::new(AtomicBool::new(false)));
    let _events = runtime
        .spawn(first, first_session, request(), identity())
        .await
        .unwrap();
    let duplicate_shutdown = Arc::new(AtomicBool::new(false));
    let (duplicate_session, _) = session(Arc::clone(&duplicate_shutdown));
    let duplicate = runtime
        .spawn(first, duplicate_session, request(), identity())
        .await
        .err()
        .expect("duplicate operation is rejected");
    assert_eq!(duplicate.reason(), DriverRuntimeError::DuplicateOperation);
    assert_eq!(duplicate.shutdown_error(), None);
    // Rejected spawns no longer consume the session.
    assert!(!duplicate_shutdown.load(Ordering::SeqCst));
    let overflow_shutdown = Arc::new(AtomicBool::new(false));
    let (overflow_session, _) = session(Arc::clone(&overflow_shutdown));
    let overflow = runtime
        .spawn(second, overflow_session, request(), identity())
        .await
        .err()
        .expect("capacity overflow is rejected");
    assert_eq!(overflow.reason(), DriverRuntimeError::CapacityExhausted);
    assert_eq!(overflow.shutdown_error(), None);
    assert!(!overflow_shutdown.load(Ordering::SeqCst));
    let failing: Arc<dyn DriverSession> = Arc::new(ControlledSession {
        release: Arc::new(Notify::new()),
        released: Arc::new(AtomicBool::new(false)),
        started: Arc::new(AtomicBool::new(false)),
        shutdown: Arc::new(AtomicBool::new(false)),
        fail_shutdown: true,
    });
    let failed_cleanup = runtime
        .spawn(second, failing, request(), identity())
        .await
        .err()
        .expect("capacity rejection keeps the session for the caller");
    assert_eq!(failed_cleanup.shutdown_error(), None);
    assert_eq!(
        runtime.cancel(second),
        RuntimeCancelOutcome::UnknownOperation
    );
    runtime.shutdown().await.unwrap();
}
