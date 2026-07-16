use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tablerock_core::{
    BoundedText, ByteLimit, CancelDispatch, ColumnMetadata, CommandBudget, CommandBudgetLimits,
    CommandEnvelope, CommandIntent, CommandScope, ContextId, Engine, EngineType, IdParts,
    OperationId, OperationOutcome, OperationPhase, OperationScope, OwnedValue, PageDelivery,
    PageFacts, PageIdentity, PageLimits, PageWarnings, ProfileId, RequestId, ResultId, ResultPage,
    Revision, RowTotal, ServiceCoordinator, ServiceLimits, SessionId, ShutdownMode,
    ShutdownOutcome, Truncation,
};
use tablerock_engine::{
    AdapterError, DriverFuture, DriverPageRequest, DriverPageStream, DriverRuntime, DriverSession,
    EngineService, EngineServiceError, EngineServiceUpdate, PostgresProbeQuery, RuntimeStopOutcome,
};

struct OnePageStream(Option<ResultPage>);

impl DriverPageStream for OnePageStream {
    fn next_page<'a>(
        &'a mut self,
        _identity: PageIdentity,
        _start_row: u64,
    ) -> DriverFuture<'a, Result<Option<ResultPage>, AdapterError>> {
        Box::pin(async move { Ok(self.0.take()) })
    }
}

struct PageSession {
    page: ResultPage,
    shutdown: Arc<AtomicBool>,
}

struct PanicSession;

struct HoldStream;

impl DriverPageStream for HoldStream {
    fn next_page<'a>(
        &'a mut self,
        _identity: PageIdentity,
        _start_row: u64,
    ) -> DriverFuture<'a, Result<Option<ResultPage>, AdapterError>> {
        Box::pin(std::future::pending())
    }
}

struct HoldSession(Arc<AtomicBool>);

impl DriverSession for HoldSession {
    fn engine(&self) -> Engine {
        Engine::PostgreSql
    }

    fn start_page_stream<'a>(
        &'a self,
        _request: DriverPageRequest,
    ) -> DriverFuture<'a, Result<Box<dyn DriverPageStream>, AdapterError>> {
        Box::pin(async { Ok(Box::new(HoldStream) as Box<dyn DriverPageStream>) })
    }

    fn cancel<'a>(&'a self, _operation_id: OperationId) -> DriverFuture<'a, CancelDispatch> {
        Box::pin(async { CancelDispatch::Unsupported })
    }

    fn shutdown(self: Box<Self>) -> DriverFuture<'static, Result<(), AdapterError>> {
        Box::pin(async move {
            self.0.store(true, Ordering::SeqCst);
            Ok(())
        })
    }
}

impl DriverSession for PanicSession {
    fn engine(&self) -> Engine {
        Engine::PostgreSql
    }

    fn start_page_stream<'a>(
        &'a self,
        _request: DriverPageRequest,
    ) -> DriverFuture<'a, Result<Box<dyn DriverPageStream>, AdapterError>> {
        Box::pin(async { panic!("simulated driver task panic") })
    }

    fn cancel<'a>(&'a self, _operation_id: OperationId) -> DriverFuture<'a, CancelDispatch> {
        Box::pin(async { CancelDispatch::Unsupported })
    }

    fn shutdown(self: Box<Self>) -> DriverFuture<'static, Result<(), AdapterError>> {
        Box::pin(async { Ok(()) })
    }
}

impl DriverSession for PageSession {
    fn engine(&self) -> Engine {
        Engine::PostgreSql
    }

    fn start_page_stream<'a>(
        &'a self,
        _request: DriverPageRequest,
    ) -> DriverFuture<'a, Result<Box<dyn DriverPageStream>, AdapterError>> {
        let page = self.page.clone();
        Box::pin(
            async move { Ok(Box::new(OnePageStream(Some(page))) as Box<dyn DriverPageStream>) },
        )
    }

    fn cancel<'a>(&'a self, _operation_id: OperationId) -> DriverFuture<'a, CancelDispatch> {
        Box::pin(async { CancelDispatch::Unsupported })
    }

    fn shutdown(self: Box<Self>) -> DriverFuture<'static, Result<(), AdapterError>> {
        Box::pin(async move {
            self.shutdown.store(true, Ordering::SeqCst);
            Ok(())
        })
    }
}

fn opaque<T>(
    low: u64,
    build: impl FnOnce(IdParts) -> Result<T, tablerock_core::IdDecodeError>,
) -> T {
    build(IdParts::new(0, low).unwrap()).unwrap()
}

fn operation(low: u64) -> OperationId {
    opaque(low, OperationId::from_parts)
}

fn scope() -> OperationScope {
    OperationScope::new(
        opaque(1, ProfileId::from_parts),
        opaque(2, SessionId::from_parts),
        opaque(3, ContextId::from_parts),
    )
}

fn configured_core(max_operations: u32) -> ServiceCoordinator {
    let scope = scope();
    let mut core = ServiceCoordinator::new(ServiceLimits::new(8, max_operations, 2, 8).unwrap());
    core.register_scope(CommandScope::Profile(scope.profile_id()), Revision::INITIAL)
        .unwrap();
    core.register_scope(
        CommandScope::Session {
            profile_id: scope.profile_id(),
            session_id: scope.session_id(),
        },
        Revision::INITIAL,
    )
    .unwrap();
    core.register_scope(CommandScope::Context(scope), Revision::INITIAL)
        .unwrap();
    core
}

fn command(seed: u64) -> CommandEnvelope {
    let budget = CommandBudget::new(10_000, 8, 1024, 128)
        .unwrap()
        .validate(CommandBudgetLimits::new(10_000, 8, 1024, 128).unwrap())
        .unwrap();
    CommandEnvelope::new(
        opaque(seed, RequestId::from_parts),
        CommandScope::Context(scope()),
        Revision::INITIAL,
        budget,
        None,
        CommandIntent::RefreshCatalog,
    )
    .unwrap()
}

fn page_identity() -> PageIdentity {
    PageIdentity::new(
        opaque(10, ResultId::from_parts),
        Revision::INITIAL,
        Engine::PostgreSql,
    )
}

fn request() -> DriverPageRequest {
    DriverPageRequest::PostgreSqlProbe {
        query: PostgresProbeQuery::BoundedSeries,
        limits: PageLimits::new(8, 8, 1024, 128),
        max_cell_bytes: 128,
    }
}

fn page() -> ResultPage {
    let text = |value: &str| BoundedText::copy_from_str(value, ByteLimit::new(16)).unwrap();
    ResultPage::from_row_major(
        page_identity(),
        0,
        RowTotal::Known(1),
        PageFacts::new(PageDelivery::Final, PageWarnings::none()),
        vec![ColumnMetadata::new(
            text("value"),
            EngineType::new(Engine::PostgreSql, text("text")).unwrap(),
            false,
        )],
        vec![OwnedValue::text(text("alpha"), Truncation::Complete).unwrap()],
        PageLimits::new(8, 8, 1024, 128),
    )
    .unwrap()
}

fn session(shutdown: Arc<AtomicBool>) -> Box<dyn DriverSession> {
    Box::new(PageSession {
        page: page(),
        shutdown,
    })
}

#[tokio::test]
async fn maps_runtime_pages_and_exit_into_core_lifecycle() {
    let operation_id = operation(1);
    let shutdown = Arc::new(AtomicBool::new(false));
    let mut service = EngineService::new(configured_core(2), DriverRuntime::new(2, 4).unwrap());
    service
        .submit(
            operation_id,
            command(100),
            session(Arc::clone(&shutdown)),
            request(),
            page_identity(),
        )
        .await
        .unwrap();

    assert!(matches!(
        service.next_update(operation_id).await.unwrap(),
        Some(EngineServiceUpdate::Started)
    ));
    assert_eq!(
        service.core().operation_phase(operation_id),
        Some(OperationPhase::Running)
    );
    assert!(matches!(
        service.next_update(operation_id).await.unwrap(),
        Some(EngineServiceUpdate::Page(_))
    ));
    assert_eq!(
        service.core().operation_phase(operation_id),
        Some(OperationPhase::Streaming)
    );
    assert!(matches!(
        service.next_update(operation_id).await.unwrap(),
        Some(EngineServiceUpdate::Terminal(OperationOutcome::Completed))
    ));
    assert_eq!(
        service.core().operation_phase(operation_id),
        Some(OperationPhase::Terminal(OperationOutcome::Completed))
    );
    assert!(shutdown.load(Ordering::SeqCst));
}

#[tokio::test]
async fn immediate_cancel_never_regresses_cancel_requested_to_running() {
    let operation_id = operation(1);
    let mut service = EngineService::new(configured_core(1), DriverRuntime::new(1, 4).unwrap());
    service
        .submit(
            operation_id,
            command(100),
            session(Arc::new(AtomicBool::new(false))),
            request(),
            page_identity(),
        )
        .await
        .unwrap();
    service.cancel(operation_id).unwrap();
    assert_eq!(
        service.core().operation_phase(operation_id),
        Some(OperationPhase::CancelRequested)
    );

    loop {
        if let Some(EngineServiceUpdate::Terminal(outcome)) =
            service.next_update(operation_id).await.unwrap()
        {
            assert_eq!(outcome, OperationOutcome::CompletedBeforeCancel);
            break;
        }
        assert_eq!(
            service.core().operation_phase(operation_id),
            Some(OperationPhase::CancelRequested)
        );
    }
}

#[tokio::test]
async fn core_rejection_consumes_session_shutdown() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let mut core = configured_core(1);
    core.submit(operation(1), command(100)).unwrap();
    let mut service = EngineService::new(core, DriverRuntime::new(1, 2).unwrap());
    let error = service
        .submit(
            operation(2),
            command(101),
            session(Arc::clone(&shutdown)),
            request(),
            page_identity(),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, EngineServiceError::CoreSubmission { .. }));
    assert!(shutdown.load(Ordering::SeqCst));
}

#[tokio::test]
async fn runtime_panic_becomes_unknown_instead_of_leaving_core_active() {
    let operation_id = operation(1);
    let mut service = EngineService::new(configured_core(1), DriverRuntime::new(1, 2).unwrap());
    service
        .submit(
            operation_id,
            command(100),
            Box::new(PanicSession),
            request(),
            page_identity(),
        )
        .await
        .unwrap();
    assert!(matches!(
        service.next_update(operation_id).await,
        Err(EngineServiceError::Runtime(_))
    ));
    assert_eq!(
        service.core().operation_phase(operation_id),
        Some(OperationPhase::Terminal(OperationOutcome::Unknown))
    );
}

#[tokio::test]
async fn cancel_active_shutdown_drains_as_client_stopped_then_releases_runtime() {
    let operation_id = operation(1);
    let shutdown = Arc::new(AtomicBool::new(false));
    let mut service = EngineService::new(configured_core(1), DriverRuntime::new(1, 1).unwrap());
    service
        .submit(
            operation_id,
            command(100),
            Box::new(HoldSession(Arc::clone(&shutdown))),
            request(),
            page_identity(),
        )
        .await
        .unwrap();

    let outcome = service.begin_shutdown(ShutdownMode::CancelActive).unwrap();
    assert_eq!(
        outcome.core,
        ShutdownOutcome::Draining {
            active_operations: 1
        }
    );
    assert_eq!(
        outcome.client_stops.as_ref(),
        &[(operation_id, RuntimeStopOutcome::Requested)]
    );
    assert!(matches!(
        service.complete_shutdown().await,
        Err(EngineServiceError::ShutdownStillDraining)
    ));

    loop {
        if let Some(EngineServiceUpdate::Terminal(outcome)) =
            service.next_update(operation_id).await.unwrap()
        {
            assert_eq!(outcome, OperationOutcome::ClientStopped);
            break;
        }
    }
    assert_eq!(
        service.core().phase(),
        tablerock_core::ServicePhase::Stopped
    );
    service.complete_shutdown().await.unwrap();
    assert!(shutdown.load(Ordering::SeqCst));
}

#[tokio::test]
async fn graceful_shutdown_allows_completion_without_client_stop() {
    let operation_id = operation(1);
    let mut service = EngineService::new(configured_core(1), DriverRuntime::new(1, 2).unwrap());
    service
        .submit(
            operation_id,
            command(100),
            session(Arc::new(AtomicBool::new(false))),
            request(),
            page_identity(),
        )
        .await
        .unwrap();
    let outcome = service.begin_shutdown(ShutdownMode::Graceful).unwrap();
    assert!(outcome.client_stops.is_empty());
    assert!(matches!(outcome.core, ShutdownOutcome::Draining { .. }));

    loop {
        if let Some(EngineServiceUpdate::Terminal(outcome)) =
            service.next_update(operation_id).await.unwrap()
        {
            assert_eq!(outcome, OperationOutcome::Completed);
            break;
        }
    }
    service.complete_shutdown().await.unwrap();
}
