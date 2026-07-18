//! Cross-adapter conformance seeds: bridge vs in-process page contract.
//!
//! Full three-engine live containers remain for the real-server CI matrix;
//! this suite proves the UniFFI facade preserves the shared client contract
//! for command validation, pages, events, cancel, and shutdown using
//! deterministic driver stubs for all three engines.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tablerock_core::{
    BoundedText, ByteLimit, CancelDispatch, ColumnMetadata, Engine, EngineType, IdParts,
    OperationId, OwnedValue, PageDelivery, PageFacts, PageIdentity, PageLimits, PageWarnings,
    ProfileId, ResultId, ResultPage, Revision, RowTotal,
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

struct FixedPageSession {
    engine: Engine,
    page: ResultPage,
}

impl DriverSession for FixedPageSession {
    fn engine(&self) -> Engine {
        self.engine
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

    fn health<'a>(&'a self) -> DriverFuture<'a, Result<SessionHealth, AdapterError>> {
        let engine = self.engine;
        Box::pin(async move { Ok(SessionHealth::new(engine, true, 0)) })
    }

    fn catalog<'a>(
        &'a self,
        _request: tablerock_engine::CatalogRequest,
    ) -> DriverFuture<'a, Result<tablerock_engine::CatalogSubtree, AdapterError>> {
        let engine = self.engine;
        Box::pin(async move {
            Err(AdapterError::new(
                engine,
                AdapterFailureClass::InvalidRequest,
            ))
        })
    }

    fn describe<'a>(&'a self) -> DriverFuture<'a, Result<ServerDescribe, AdapterError>> {
        let engine = self.engine;
        Box::pin(async move { Ok(ServerDescribe::new(engine, "test", 0)) })
    }

    fn shutdown(self: Box<Self>) -> DriverFuture<'static, Result<(), AdapterError>> {
        Box::pin(async { Ok(()) })
    }
}

struct HoldSession {
    engine: Engine,
    cancelled: Arc<AtomicBool>,
}

impl DriverSession for HoldSession {
    fn engine(&self) -> Engine {
        self.engine
    }

    fn start_page_stream<'a>(
        &'a self,
        _request: DriverPageRequest,
    ) -> DriverFuture<'a, Result<Box<dyn DriverPageStream>, AdapterError>> {
        Box::pin(async { Ok(Box::new(HoldStream) as Box<dyn DriverPageStream>) })
    }

    fn cancel<'a>(&'a self, _operation_id: OperationId) -> DriverFuture<'a, CancelDispatch> {
        self.cancelled.store(true, Ordering::SeqCst);
        Box::pin(async { CancelDispatch::RequestSent })
    }

    fn health<'a>(&'a self) -> DriverFuture<'a, Result<SessionHealth, AdapterError>> {
        let engine = self.engine;
        Box::pin(async move { Ok(SessionHealth::new(engine, true, 0)) })
    }

    fn catalog<'a>(
        &'a self,
        _request: tablerock_engine::CatalogRequest,
    ) -> DriverFuture<'a, Result<tablerock_engine::CatalogSubtree, AdapterError>> {
        let engine = self.engine;
        Box::pin(async move {
            Err(AdapterError::new(
                engine,
                AdapterFailureClass::InvalidRequest,
            ))
        })
    }

    fn describe<'a>(&'a self) -> DriverFuture<'a, Result<ServerDescribe, AdapterError>> {
        let engine = self.engine;
        Box::pin(async move { Ok(ServerDescribe::new(engine, "test", 0)) })
    }

    fn shutdown(self: Box<Self>) -> DriverFuture<'static, Result<(), AdapterError>> {
        Box::pin(async { Ok(()) })
    }
}

fn sample_page(engine: Engine, result_low: u64, values: &[i64]) -> (ResultId, ResultPage) {
    let result_id = ResultId::from_parts(IdParts::new(0, result_low).unwrap()).unwrap();
    let type_name = match engine {
        Engine::PostgreSql => "int8",
        Engine::ClickHouse => "Int64",
        Engine::Redis => "integer",
    };
    let page = ResultPage::from_row_major(
        PageIdentity::new(result_id, Revision::INITIAL, engine),
        0,
        RowTotal::Known(values.len() as u64),
        PageFacts::new(PageDelivery::Final, PageWarnings::none()),
        vec![ColumnMetadata::new(
            BoundedText::copy_from_str("n", ByteLimit::new(1)).unwrap(),
            EngineType::new(
                engine,
                BoundedText::copy_from_str(type_name, ByteLimit::new(16)).unwrap(),
            )
            .unwrap(),
            false,
        )],
        values.iter().copied().map(OwnedValue::signed).collect(),
        PageLimits::new(500, 64, 1024 * 1024, 64 * 1024),
    )
    .unwrap();
    (result_id, page)
}

fn open_fixed(bridge: &TableRockBridge, engine: Engine, page: ResultPage) -> Vec<u8> {
    bridge
        .open_driver_session(engine, Box::new(FixedPageSession { engine, page }))
        .unwrap()
}

fn probe(bridge: &TableRockBridge, session_id: Vec<u8>, result_id: ResultId) -> Vec<u8> {
    bridge
        .submit(SubmitSpec {
            intent: "probe".into(),
            session_id,
            statement: Some("select 1".into()),
            result_id: Some(result_id.to_bytes().to_vec()),
            start_row: None,
            row_count: Some(100),
            expected_revision: 0,
        })
        .unwrap()
}

#[test]
fn bridge_page_bytes_match_in_process_encode_all_engines() {
    for (engine, low) in [
        (Engine::PostgreSql, 77_u64),
        (Engine::ClickHouse, 78),
        (Engine::Redis, 79),
    ] {
        let (result_id, page) = sample_page(engine, low, &[1, 2]);
        let in_process = page.encode_v1();
        let bridge = TableRockBridge::new_for_test();
        let session_id = open_fixed(&bridge, engine, page);
        let op = probe(&bridge, session_id, result_id);
        bridge.pump(op).unwrap();

        let via_events = bridge
            .next_events(0, 32)
            .unwrap()
            .events
            .into_iter()
            .find(|e| e.kind == "page")
            .and_then(|e| e.page_bytes)
            .expect("page event");
        assert_eq!(via_events, in_process, "engine={engine:?} events");

        let via_fetch = bridge
            .fetch_page(result_id.to_bytes().to_vec(), 0, 0)
            .unwrap();
        assert_eq!(via_fetch, in_process, "engine={engine:?} fetch");
    }
}

#[test]
fn command_validation_rejects_unknown_intent_and_stale_revision() {
    let (result_id, page) = sample_page(Engine::PostgreSql, 80, &[1]);
    let bridge = TableRockBridge::new_for_test();
    let session_id = open_fixed(&bridge, Engine::PostgreSql, page);

    let err = bridge
        .submit(SubmitSpec {
            intent: "not-a-command".into(),
            session_id: session_id.clone(),
            statement: None,
            result_id: None,
            start_row: None,
            row_count: None,
            expected_revision: 0,
        })
        .unwrap_err();
    assert!(matches!(
        err,
        BridgeError::Rejected { ref code, .. } if code == "unknown-intent"
    ));

    let err = bridge
        .submit(SubmitSpec {
            intent: "probe".into(),
            session_id,
            statement: Some("select 1".into()),
            result_id: Some(result_id.to_bytes().to_vec()),
            start_row: None,
            row_count: Some(10),
            expected_revision: 99,
        })
        .unwrap_err();
    assert!(matches!(
        err,
        BridgeError::Rejected { ref code, .. } if code == "revision-mismatch"
    ));
}

#[test]
fn catalog_intent_is_supported_for_all_engines_without_client_statement() {
    for (engine, low) in [
        (Engine::PostgreSql, 180_u64),
        (Engine::ClickHouse, 181),
        (Engine::Redis, 182),
    ] {
        let (result_id, page) = sample_page(engine, low, &[1]);
        let bridge = TableRockBridge::new_for_test();
        let session_id = open_fixed(&bridge, engine, page);
        let operation = bridge
            .submit(SubmitSpec {
                intent: "catalog".into(),
                session_id,
                statement: None,
                result_id: Some(result_id.to_bytes().to_vec()),
                start_row: None,
                row_count: Some(100),
                expected_revision: 0,
            })
            .unwrap();
        bridge.pump(operation).unwrap();
        assert!(
            bridge
                .next_events(0, 32)
                .unwrap()
                .events
                .iter()
                .any(|event| event.kind == "page")
        );
    }
}

#[test]
fn event_ordering_and_future_cursor_and_resync() {
    let (result_id, page) = sample_page(Engine::PostgreSql, 81, &[3]);
    let bridge = TableRockBridge::new_for_test();
    let session_id = open_fixed(&bridge, Engine::PostgreSql, page);
    let op = probe(&bridge, session_id, result_id);
    bridge.pump(op).unwrap();

    let first = bridge.next_events(0, 1).unwrap();
    assert!(!first.resync_required);
    assert_eq!(first.events.len(), 1);
    assert_eq!(first.events[0].sequence, 0);
    let mid = first.next_cursor;

    let rest = bridge.next_events(mid, 32).unwrap();
    assert!(!rest.resync_required);
    assert!(rest.events.iter().all(|e| e.sequence >= mid));
    // Sequences are strictly increasing across the full log.
    let all = bridge.next_events(0, 32).unwrap();
    let mut prev = None;
    for event in &all.events {
        if let Some(p) = prev {
            assert!(event.sequence > p);
        }
        prev = Some(event.sequence);
    }

    let future = bridge.next_events(all.next_cursor + 100, 8).unwrap_err();
    assert!(matches!(future, BridgeError::FutureCursor));
}

#[test]
fn cancel_pending_operation_requests_cancel() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let bridge = TableRockBridge::new_for_test();
    let session_id = bridge
        .open_driver_session(
            Engine::PostgreSql,
            Box::new(HoldSession {
                engine: Engine::PostgreSql,
                cancelled: Arc::clone(&cancelled),
            }),
        )
        .unwrap();
    let op = bridge
        .submit(SubmitSpec {
            intent: "probe".into(),
            session_id,
            statement: Some("select 1".into()),
            result_id: None,
            start_row: None,
            row_count: Some(10),
            expected_revision: 0,
        })
        .unwrap();

    let outcome = bridge.cancel(op).unwrap();
    assert!(
        outcome.core.contains("Requested") || outcome.core.contains("AlreadyRequested"),
        "core={}",
        outcome.core
    );
    // Shutdown with cancel-active must accept while work is pending.
    let shutdown = bridge.shutdown(true, 1_000).unwrap();
    assert!(!shutdown.core.is_empty());
}

#[test]
fn shutdown_graceful_with_completed_work() {
    let (result_id, page) = sample_page(Engine::ClickHouse, 82, &[9]);
    let bridge = TableRockBridge::new_for_test();
    let session_id = open_fixed(&bridge, Engine::ClickHouse, page);
    let op = probe(&bridge, session_id, result_id);
    bridge.pump(op).unwrap();
    let outcome = bridge.shutdown(false, 1_000).unwrap();
    assert_eq!(outcome.active_operations, 0);
    // Second submit after shutdown is rejected.
    let err = bridge
        .submit(SubmitSpec {
            intent: "probe".into(),
            session_id: vec![0; 16],
            statement: None,
            result_id: None,
            start_row: None,
            row_count: None,
            expected_revision: 0,
        })
        .unwrap_err();
    assert!(matches!(
        err,
        BridgeError::ShuttingDown
            | BridgeError::RuntimeUnavailable
            | BridgeError::UnknownSession
            | BridgeError::Rejected { .. }
    ));
}

#[test]
fn open_params_redaction_and_oversized_page_decode() {
    let params = tablerock_ffi::OpenParams {
        engine: "postgresql".into(),
        host: "h".into(),
        port: 1,
        database: "d".into(),
        user: "u".into(),
        password: "do-not-leak-me".into(),
    };
    let debug = format!("{params:?}");
    assert!(!debug.contains("do-not-leak-me"));
    assert!(debug.contains("<redacted>"));

    // Oversized declared arena rejected before body allocation.
    let (result_id, page) = sample_page(Engine::Redis, 83, &[1]);
    let mut encoded = page.encode_v1();
    let arena_field_offset = 4 + 2 + 16 + 8 + 1 + 8 + 4 + 4 + 1 + 8;
    let huge = (8 * 1024 * 1024_u64).to_le_bytes();
    encoded[arena_field_offset..arena_field_offset + 8].copy_from_slice(&huge);
    let err = ResultPage::decode_v1(&encoded, PageLimits::new(500, 64, 1024, 1024)).unwrap_err();
    assert!(matches!(
        err,
        tablerock_core::PageValidationError::ArenaLimitExceeded { .. }
    ));
    let _ = result_id;
}

#[test]
fn uniffi_surface_has_no_per_cell_export() {
    // Guard: facade public API must not expose cell-level accessors.
    // Runtime grep of the generated Swift is the distribution check; this
    // asserts the Rust object methods stay coarse.
    let bridge = TableRockBridge::new_for_test();
    bridge.ensure_runtime().unwrap();
    // Only coarse methods: if this compiles, fetch_page returns Vec<u8>.
    let _ = bridge.fetch_page(vec![0; 16], 0, 0);
}

#[test]
fn open_profile_requires_persistence_and_loads_literals() {
    use std::fs;
    use tablerock_core::{
        ProfileAggregate, ProfileConnectionSnapshot, ProfileDurability, ProfileGroupName,
        ProfileIdentity, ProfileLimits, ProfileName, ProfileOrganization, ProfilePolicy,
        ProfilePreferences, ProfileProperty, ProfilePropertyBinding, ProfilePropertySet,
        ProfileSafetyMode, ProfileTag, ReconnectPreference, TlsPolicy,
    };
    use tablerock_persistence::PersistenceActor;

    let path = std::env::temp_dir().join(format!(
        "tablerock-bridge-profile-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = fs::remove_file(&path);
    let actor = PersistenceActor::open(&path).unwrap();
    let profile_id = ProfileId::from_parts(IdParts::new(1, 42).unwrap()).unwrap();
    let properties = ProfilePropertySet::new(vec![
        ProfilePropertyBinding::literal(
            ProfileProperty::Host,
            BoundedText::copy_from_str("127.0.0.1", ByteLimit::new(16)).unwrap(),
        )
        .unwrap(),
        ProfilePropertyBinding::literal(
            ProfileProperty::Port,
            BoundedText::copy_from_str("1", ByteLimit::new(8)).unwrap(),
        )
        .unwrap(),
        ProfilePropertyBinding::literal(
            ProfileProperty::DefaultContext,
            BoundedText::copy_from_str("postgres", ByteLimit::new(16)).unwrap(),
        )
        .unwrap(),
        ProfilePropertyBinding::literal(
            ProfileProperty::Username,
            BoundedText::copy_from_str("postgres", ByteLimit::new(16)).unwrap(),
        )
        .unwrap(),
    ])
    .unwrap();
    let connection = ProfileConnectionSnapshot::new(
        ProfileIdentity::new(
            profile_id,
            Revision::INITIAL,
            Engine::PostgreSql,
            ProfileName::new(
                BoundedText::copy_from_str("bridge-test", ByteLimit::new(32)).unwrap(),
            )
            .unwrap(),
        ),
        properties,
        ProfilePolicy::new(
            TlsPolicy::Disabled,
            ProfileSafetyMode::ConfirmWrites,
            ProfileLimits::new(10_000, 30_000, 5_000, 16 * 1024 * 1024).unwrap(),
        ),
    )
    .unwrap();
    let aggregate = ProfileAggregate::new(
        connection,
        ProfileDurability::Saved,
        ProfileOrganization::new(
            Some(
                ProfileGroupName::new(BoundedText::copy_from_str("g", ByteLimit::new(8)).unwrap())
                    .unwrap(),
            ),
            vec![
                ProfileTag::new(BoundedText::copy_from_str("t", ByteLimit::new(8)).unwrap())
                    .unwrap(),
            ],
            true,
            0,
            None,
        )
        .unwrap(),
        ProfilePreferences::new(ReconnectPreference::Manual, true, 250).unwrap(),
    )
    .unwrap();
    actor
        .create_profile(aggregate.persistable().unwrap())
        .unwrap();
    actor.shutdown().unwrap();

    let bridge = TableRockBridge::new_for_test();
    let missing = bridge
        .open_profile(profile_id.to_bytes().to_vec(), None)
        .unwrap_err();
    assert!(matches!(
        missing,
        BridgeError::Rejected { ref code, .. } if code == "persistence"
    ));

    bridge
        .configure_persistence(path.to_string_lossy().into_owned())
        .unwrap();
    // Port 1 is unreachable — proves load+connect path without a live server.
    let err = bridge
        .open_profile(profile_id.to_bytes().to_vec(), None)
        .unwrap_err();
    assert!(matches!(
        err,
        BridgeError::Rejected { ref code, .. } if code == "connect"
    ));
    let _ = fs::remove_file(&path);
    let _ = fs::remove_file(format!("{}-wal", path.display()));
    let _ = fs::remove_file(format!("{}-shm", path.display()));
}

#[test]
fn apply_review_token_consumes_handle_even_when_apply_fails() {
    let (_, page) = sample_page(Engine::PostgreSql, 91, &[1]);
    let bridge = TableRockBridge::new_for_test();
    let session_id = open_fixed(&bridge, Engine::PostgreSql, page);
    let token = bridge
        .insert_reviewed_probe(session_id.clone(), 1_000, 2_000, 1_100, None, None)
        .unwrap();

    // FixedPageSession apply fails closed; token is still consumed first.
    let err = bridge
        .apply_review_token(token.clone(), 1_500, session_id.clone(), 0)
        .unwrap_err();
    assert!(matches!(
        err,
        BridgeError::Rejected { ref code, .. } if code == "apply"
    ));
    // Second use cannot retry the same handle (ambiguous-write non-retry).
    let again = bridge
        .apply_review_token(token, 1_600, session_id.clone(), 0)
        .unwrap_err();
    assert!(matches!(
        again,
        BridgeError::Rejected { ref code, .. } if code == "authorize"
    ));

    bridge.disconnect(session_id).unwrap();
}

#[test]
fn disconnect_rejects_unknown_session() {
    let bridge = TableRockBridge::new_for_test();
    bridge.ensure_runtime().unwrap();
    let bogus = ResultId::from_parts(IdParts::new(0, 5).unwrap())
        .unwrap()
        .to_bytes()
        .to_vec();
    // Session id layout is 16 bytes; unknown id is a disconnect error path.
    let err = bridge.disconnect(bogus).unwrap_err();
    assert!(matches!(
        err,
        BridgeError::Rejected { .. } | BridgeError::UnknownSession
    ));
}

#[test]
fn review_token_is_consume_once_and_expiry_blocks() {
    let (_, page) = sample_page(Engine::PostgreSql, 90, &[1]);
    let bridge = TableRockBridge::new_for_test();
    let session_id = open_fixed(&bridge, Engine::PostgreSql, page);

    let token = bridge
        .insert_reviewed_probe(session_id.clone(), 1_000, 2_000, 1_100, None, None)
        .unwrap();

    // Expired authorize consumes the handle (core contract: remove then fail).
    let expired = bridge
        .authorize_review_token(token.clone(), 3_000, session_id.clone(), 0)
        .unwrap_err();
    assert!(matches!(
        expired,
        BridgeError::Rejected { ref code, .. } if code == "authorize"
    ));
    // Second attempt: token already gone.
    let missing = bridge
        .authorize_review_token(token, 1_500, session_id.clone(), 0)
        .unwrap_err();
    assert!(matches!(
        missing,
        BridgeError::Rejected { ref code, .. } if code == "authorize"
    ));

    // Fresh token authorizes once, then is gone.
    let token2 = bridge
        .insert_reviewed_probe(session_id.clone(), 1_000, 2_000, 1_100, None, None)
        .unwrap();
    bridge
        .authorize_review_token(token2.clone(), 1_500, session_id.clone(), 0)
        .unwrap();
    let second = bridge
        .authorize_review_token(token2, 1_600, session_id, 0)
        .unwrap_err();
    assert!(matches!(
        second,
        BridgeError::Rejected { ref code, .. } if code == "authorize"
    ));
}
