//! Facade unit tests: runtime lifecycle, panic containment, page encode path.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tablerock_core::{
    BoundedText, ByteLimit, CancelDispatch, ColumnMetadata, Engine, EngineType, IdParts,
    OperationId, OwnedValue, PageDelivery, PageFacts, PageIdentity, PageLimits, PageWarnings,
    ResultId, ResultPage, Revision, RowTotal,
};
use tablerock_engine::{
    AdapterError, AdapterFailureClass, DriverFuture, DriverPageRequest, DriverPageStream,
    DriverSession, ServerDescribe, SessionHealth,
};
use tablerock_ffi::{BridgeError, SubmitSpec, TableRockBridge};

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

struct FixedPageSession {
    page: ResultPage,
    shutdown: Arc<AtomicBool>,
}

impl DriverSession for FixedPageSession {
    fn engine(&self) -> Engine {
        Engine::PostgreSql
    }

    fn start_page_stream<'a>(
        &'a self,
        _request: DriverPageRequest,
    ) -> DriverFuture<'a, Result<Box<dyn DriverPageStream>, AdapterError>> {
        let page = self.page.clone();
        Box::pin(async move { Ok(Box::new(OnePageStream(Some(page))) as Box<dyn DriverPageStream>) })
    }

    fn cancel<'a>(&'a self, _operation_id: OperationId) -> DriverFuture<'a, CancelDispatch> {
        Box::pin(async { CancelDispatch::Unsupported })
    }

    fn health<'a>(&'a self) -> DriverFuture<'a, Result<SessionHealth, AdapterError>> {
        Box::pin(async {
            Ok(SessionHealth::new(Engine::PostgreSql, true, 0))
        })
    }

    fn catalog<'a>(
        &'a self,
        _request: tablerock_engine::CatalogRequest,
    ) -> DriverFuture<'a, Result<tablerock_engine::CatalogSubtree, AdapterError>> {
        Box::pin(async {
            Err(AdapterError::new(
                Engine::PostgreSql,
                AdapterFailureClass::InvalidRequest,
            ))
        })
    }

    fn describe<'a>(&'a self) -> DriverFuture<'a, Result<ServerDescribe, AdapterError>> {
        Box::pin(async {
            Ok(ServerDescribe::new(Engine::PostgreSql, "test", 0))
        })
    }

    fn shutdown(self: Box<Self>) -> DriverFuture<'static, Result<(), AdapterError>> {
        Box::pin(async move {
            self.shutdown.store(true, Ordering::SeqCst);
            Ok(())
        })
    }
}

fn sample_page(result_id: ResultId) -> ResultPage {
    let columns = vec![ColumnMetadata::new(
        BoundedText::copy_from_str("n", ByteLimit::new(1)).unwrap(),
        EngineType::new(
            Engine::PostgreSql,
            BoundedText::copy_from_str("int8", ByteLimit::new(4)).unwrap(),
        )
        .unwrap(),
        false,
    )];
    ResultPage::from_row_major(
        PageIdentity::new(result_id, Revision::INITIAL, Engine::PostgreSql),
        0,
        RowTotal::Known(1),
        PageFacts::new(PageDelivery::Final, PageWarnings::none()),
        columns,
        vec![OwnedValue::signed(42)],
        PageLimits::new(500, 64, 1024 * 1024, 64 * 1024),
    )
    .unwrap()
}

#[test]
fn panic_probe_is_contained() {
    let bridge = TableRockBridge::new_for_test();
    let err = bridge.panic_probe().unwrap_err();
    assert!(matches!(err, BridgeError::ContainedPanic { .. }));
    // Process still usable after containment.
    bridge.ensure_runtime().unwrap();
    bridge.destroy_runtime().unwrap();
    bridge.destroy_runtime().unwrap(); // idempotent
}

#[test]
fn open_submit_pump_fetch_shutdown_round_trip() {
    let result_id = ResultId::from_parts(IdParts::new(0, 99).unwrap()).unwrap();
    let page = sample_page(result_id);
    let expected_bytes = page.encode_v1();
    let bridge = TableRockBridge::new_for_test();
    let session_id = bridge
        .open_driver_session(
            Engine::PostgreSql,
            Box::new(FixedPageSession {
                page,
                shutdown: Arc::new(AtomicBool::new(false)),
            }),
        )
        .unwrap();

    let operation_id = bridge
        .submit(SubmitSpec {
            intent: "probe".into(),
            session_id: session_id.clone(),
            statement: Some("select 1".into()),
            result_id: Some(result_id.to_bytes().to_vec()),
            start_row: None,
            row_count: Some(100),
            expected_revision: 0,
        })
        .unwrap();

    bridge.pump(operation_id.clone()).unwrap();

    let batch = bridge.next_events(0, 32).unwrap();
    assert!(!batch.resync_required);
    assert!(batch.events.iter().any(|e| e.kind == "started"));
    assert!(batch.events.iter().any(|e| e.kind == "page"));
    assert!(batch.events.iter().any(|e| e.kind == "terminal"));

    let page_event = batch
        .events
        .iter()
        .find(|e| e.kind == "page")
        .expect("page event");
    assert_eq!(page_event.page_bytes.as_ref().unwrap(), &expected_bytes);

    let fetched = bridge
        .fetch_page(result_id.to_bytes().to_vec(), 0, 0)
        .unwrap();
    assert_eq!(fetched, expected_bytes);

    let decoded = ResultPage::decode_v1(&fetched, PageLimits::new(500, 64, 1024 * 1024, 64 * 1024))
        .unwrap();
    assert_eq!(decoded.cell(0, 0).unwrap().bytes(), &42_i64.to_be_bytes());

    let shutdown = bridge.shutdown(false, 1_000).unwrap();
    assert!(shutdown.active_operations == 0 || !shutdown.core.is_empty());
}

#[test]
fn open_params_debug_redacts_password() {
    let params = tablerock_ffi::OpenParams {
        engine: "postgresql".into(),
        host: "127.0.0.1".into(),
        port: 5432,
        database: "db".into(),
        user: "u".into(),
        password: "super-secret".into(),
    };
    let debug = format!("{params:?}");
    assert!(!debug.contains("super-secret"));
    assert!(debug.contains("<redacted>"));
}
