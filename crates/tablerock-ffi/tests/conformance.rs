//! Cross-adapter seed: page bytes from the bridge match in-process encode_v1.

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
use tablerock_ffi::{SubmitSpec, TableRockBridge};

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
        Box::pin(async { Ok(SessionHealth::new(Engine::PostgreSql, true, 0)) })
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
        Box::pin(async { Ok(ServerDescribe::new(Engine::PostgreSql, "test", 0)) })
    }

    fn shutdown(self: Box<Self>) -> DriverFuture<'static, Result<(), AdapterError>> {
        let _ = Arc::new(AtomicBool::new(false));
        Box::pin(async { Ok(()) })
    }
}

#[test]
fn bridge_page_bytes_match_in_process_encode() {
    let result_id = ResultId::from_parts(IdParts::new(0, 77).unwrap()).unwrap();
    let page = ResultPage::from_row_major(
        PageIdentity::new(result_id, Revision::INITIAL, Engine::PostgreSql),
        0,
        RowTotal::Known(2),
        PageFacts::new(PageDelivery::Final, PageWarnings::none()),
        vec![ColumnMetadata::new(
            BoundedText::copy_from_str("n", ByteLimit::new(1)).unwrap(),
            EngineType::new(
                Engine::PostgreSql,
                BoundedText::copy_from_str("int8", ByteLimit::new(4)).unwrap(),
            )
            .unwrap(),
            false,
        )],
        vec![OwnedValue::signed(1), OwnedValue::signed(2)],
        PageLimits::new(500, 64, 1024 * 1024, 64 * 1024),
    )
    .unwrap();
    let in_process = page.encode_v1();

    let bridge = TableRockBridge::new_for_test();
    let session_id = bridge
        .open_driver_session(Engine::PostgreSql, Box::new(FixedPageSession { page }))
        .unwrap();
    let op = bridge
        .submit(SubmitSpec {
            intent: "probe".into(),
            session_id,
            statement: Some("select 1".into()),
            result_id: Some(result_id.to_bytes().to_vec()),
            start_row: None,
            row_count: Some(100),
            expected_revision: 0,
        })
        .unwrap();
    bridge.pump(op).unwrap();

    let via_events = bridge
        .next_events(0, 32)
        .unwrap()
        .events
        .into_iter()
        .find(|e| e.kind == "page")
        .and_then(|e| e.page_bytes)
        .expect("page event");
    assert_eq!(via_events, in_process);

    let via_fetch = bridge
        .fetch_page(result_id.to_bytes().to_vec(), 0, 0)
        .unwrap();
    assert_eq!(via_fetch, in_process);

    let decoded = ResultPage::decode_v1(
        &via_fetch,
        PageLimits::new(500, 64, 1024 * 1024, 64 * 1024),
    )
    .unwrap();
    assert_eq!(decoded.cell(0, 0).unwrap().bytes(), &1_i64.to_be_bytes());
    assert_eq!(decoded.cell(1, 0).unwrap().bytes(), &2_i64.to_be_bytes());
}
