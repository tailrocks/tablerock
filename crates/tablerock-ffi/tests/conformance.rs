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
    BoundedText, ByteLimit, CancelDispatch, CatalogChildrenState, CatalogNodeKind, ColumnMetadata,
    Engine, EngineType, IdParts, OperationId, OwnedValue, PageDelivery, PageFacts, PageIdentity,
    PageLimits, PageWarnings, ProfileId, ResultId, ResultPage, Revision, RowTotal,
};
use tablerock_engine::{
    AdapterError, AdapterFailureClass, CatalogExactness, CatalogNodeSeed, CatalogRequest,
    CatalogSubtree, DriverFuture, DriverPageRequest, DriverPageStream, DriverSession,
    ServerDescribe, SessionHealth,
};
use tablerock_ffi::{
    BridgeError, BridgeProfileOrderItem, BridgeSessionIntent, BridgeWorkspaceTab, SubmitSpec,
    TableRockBridge,
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
    health_failure: Option<AdapterFailureClass>,
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
        let failure = self.health_failure;
        Box::pin(async move {
            failure.map_or_else(
                || Ok(SessionHealth::new(engine, true, 7)),
                |class| Err(AdapterError::new(engine, class)),
            )
        })
    }

    fn catalog<'a>(
        &'a self,
        request: CatalogRequest,
    ) -> DriverFuture<'a, Result<CatalogSubtree, AdapterError>> {
        let engine = self.engine;
        Box::pin(async move {
            let (kind, name, children) = match request {
                CatalogRequest::PostgreSqlDatabases { .. } => (
                    CatalogNodeKind::PostgreSqlDatabase,
                    "app",
                    CatalogChildrenState::Unrequested,
                ),
                CatalogRequest::PostgreSqlSchemas { .. } => (
                    CatalogNodeKind::PostgreSqlSchema,
                    "public",
                    CatalogChildrenState::Unrequested,
                ),
                CatalogRequest::PostgreSqlRelations { .. } => (
                    CatalogNodeKind::PostgreSqlObject(tablerock_core::PostgreSqlObjectKind::Table),
                    "users",
                    CatalogChildrenState::Unrequested,
                ),
                CatalogRequest::ClickHouseDatabases { .. } => (
                    CatalogNodeKind::ClickHouseDatabase,
                    "default",
                    CatalogChildrenState::Unrequested,
                ),
                CatalogRequest::ClickHouseObjects { .. } => (
                    CatalogNodeKind::ClickHouseObject(tablerock_core::ClickHouseObjectKind::Table),
                    "events",
                    CatalogChildrenState::Unrequested,
                ),
                CatalogRequest::RedisLogicalDatabases { .. } => (
                    CatalogNodeKind::RedisLogicalDatabase,
                    "0",
                    CatalogChildrenState::Unrequested,
                ),
            };
            Ok(CatalogSubtree::new(
                engine,
                vec![CatalogNodeSeed::new(
                    kind,
                    BoundedText::copy_from_str(name, ByteLimit::new(32)).unwrap(),
                    children,
                    None,
                )],
                true,
                CatalogExactness::Exact,
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
        .open_driver_session(
            engine,
            Box::new(FixedPageSession {
                engine,
                page,
                health_failure: None,
            }),
        )
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

fn execute(
    bridge: &TableRockBridge,
    session_id: Vec<u8>,
    result_id: ResultId,
    statement: &str,
) -> Vec<u8> {
    bridge
        .submit(SubmitSpec {
            intent: "execute".into(),
            session_id,
            statement: Some(statement.into()),
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
        let health = bridge.check_session_health(session_id.clone()).unwrap();
        assert_eq!(health.state, "healthy");
        assert!(health.server_reachable);
        assert_eq!(health.elapsed_millis, Some(7));
        assert!(!health.authentication_stopped);
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
fn typed_catalog_uses_opaque_parent_handles_for_all_engines() {
    for (engine, low) in [
        (Engine::PostgreSql, 180_u64),
        (Engine::ClickHouse, 181),
        (Engine::Redis, 182),
    ] {
        let (_result_id, page) = sample_page(engine, low, &[1]);
        let bridge = TableRockBridge::new_for_test();
        let session_id = open_fixed(&bridge, engine, page);
        let roots = bridge.refresh_catalog(session_id.clone(), None).unwrap();
        assert_eq!(roots.len(), 1);
        if roots[0].expandable {
            let children = bridge
                .refresh_catalog(session_id.clone(), Some(roots[0].id_bytes.clone()))
                .unwrap();
            assert_eq!(children.len(), 1);
            assert_eq!(children[0].parent_id_bytes, Some(roots[0].id_bytes.clone()));
        }
        let stale = vec![0xff; 16];
        assert!(matches!(
            bridge.refresh_catalog(session_id, Some(stale)),
            Err(BridgeError::Rejected { ref code, .. }) if code == "unknown-catalog-node"
        ));
    }
}

#[test]
fn catalog_browse_accepts_only_cached_table_like_nodes() {
    for (engine, low) in [(Engine::PostgreSql, 183_u64), (Engine::ClickHouse, 184)] {
        let (_result_id, page) = sample_page(engine, low, &[7]);
        let bridge = TableRockBridge::new_for_test();
        let session_id = open_fixed(&bridge, engine, page);
        let roots = bridge.refresh_catalog(session_id.clone(), None).unwrap();
        let level_one = bridge
            .refresh_catalog(session_id.clone(), Some(roots[0].id_bytes.clone()))
            .unwrap();
        let object = if engine == Engine::PostgreSql {
            bridge
                .refresh_catalog(session_id.clone(), Some(level_one[0].id_bytes.clone()))
                .unwrap()
                .remove(0)
        } else {
            level_one[0].clone()
        };
        let operation = bridge
            .submit_catalog_browse(session_id.clone(), object.id_bytes.clone(), 500)
            .unwrap();
        bridge.pump(operation.clone()).unwrap();
        let events = bridge.next_events(0, 64).unwrap().events;
        let page_event = events
            .iter()
            .find(|event| {
                event.operation_id == operation && event.kind == "page" && event.rows == Some(1)
            })
            .expect("browse page");
        let encoded = page_event.page_bytes.as_ref().expect("encoded page");
        let result_id = encoded[6..22].to_vec();
        let csv = bridge
            .format_result_copy(
                result_id.clone(),
                0,
                "loaded".into(),
                None,
                None,
                "csv".into(),
            )
            .unwrap();
        assert!(csv.contains("n") && csv.contains('7'), "{csv}");
        let export_path = std::env::temp_dir().join(format!(
            "tablerock-ffi-export-{}-{low}.csv",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&export_path);
        let bytes = bridge
            .export_loaded_result(
                result_id.clone(),
                0,
                "csv".into(),
                export_path.to_string_lossy().into_owned(),
            )
            .unwrap();
        assert_eq!(bytes, csv.len() as u64);
        assert_eq!(std::fs::read_to_string(&export_path).unwrap(), csv);
        std::fs::remove_file(export_path).unwrap();
        assert!(matches!(
            bridge.export_loaded_result(
                result_id.clone(),
                0,
                "csv".into(),
                "relative.csv".into(),
            ),
            Err(BridgeError::Rejected { ref code, .. }) if code == "export-path"
        ));
        let csv_path = std::env::temp_dir().join(format!(
            "tablerock-ffi-import-{}-{low}.csv",
            std::process::id()
        ));
        std::fs::write(&csv_path, "id,name\n8,=literal\n").unwrap();
        let preview = bridge
            .preview_csv_import(csv_path.to_string_lossy().into_owned())
            .unwrap();
        assert_eq!(preview.headers, ["id", "name"]);
        assert_eq!(preview.total_rows, 1);
        assert_eq!(preview.formula_like_cells, 1);
        let review = bridge
            .stage_csv_import(
                session_id.clone(),
                object.id_bytes.clone(),
                csv_path.to_string_lossy().into_owned(),
                vec!["id".into(), "name".into()],
                vec!["signed".into(), "text".into()],
                100,
            )
            .unwrap();
        assert_eq!(review.row_count, 1);
        assert_eq!(review.column_count, 2);
        assert_eq!(review.formula_like_cells, 1);
        assert!(bridge.revoke_review_token(review.token_id).unwrap());
        std::fs::remove_file(csv_path).unwrap();
        let json = bridge
            .format_result_copy(
                result_id.clone(),
                0,
                "cell".into(),
                Some(0),
                Some(0),
                "json".into(),
            )
            .unwrap();
        assert!(json.contains(":7"), "{json}");
        let insert = bridge
            .format_result_copy(
                result_id.clone(),
                0,
                "row".into(),
                Some(0),
                None,
                "sql_insert".into(),
            )
            .unwrap();
        assert!(insert.contains("INSERT INTO"), "{insert}");
        assert!(matches!(
            bridge.format_result_copy(
                result_id, 0, "row".into(), Some(0), None, "sql_update".into()
            ),
            Err(BridgeError::Rejected { ref code, .. }) if code == "copy-format"
        ));
        assert!(matches!(
            bridge.submit_catalog_browse(session_id.clone(), roots[0].id_bytes.clone(), 500),
            Err(BridgeError::Rejected { ref code, .. }) if code == "catalog-browse-kind"
        ));
        assert!(matches!(
            bridge.submit_catalog_browse(session_id, roots[0].id_bytes.clone(), 0),
            Err(BridgeError::Rejected { ref code, .. }) if code == "catalog-browse-bounds"
        ));
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
        tls_mode: "off".into(),
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
        ProfilePreferences::new(ReconnectPreference::BoundedAutomatic, true, 250).unwrap(),
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
    assert_eq!(bridge.history_retention().unwrap(), "full");
    let listed = bridge.list_profiles().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "bridge-test");
    assert_eq!(listed[0].group.as_deref(), Some("g"));
    assert_eq!(listed[0].host.as_deref(), Some("127.0.0.1"));
    assert_eq!(listed[0].port.as_deref(), Some("1"));
    assert_eq!(listed[0].context.as_deref(), Some("postgres"));
    assert!(!listed[0].connected);
    let (history_result_id, reconnect_page) = sample_page(Engine::PostgreSql, 141, &[1]);
    let reconnect_source = bridge
        .open_driver_session_for_profile(
            profile_id,
            Engine::PostgreSql,
            Box::new(FixedPageSession {
                engine: Engine::PostgreSql,
                page: reconnect_page,
                health_failure: None,
            }),
        )
        .unwrap();
    let immediate = bridge
        .plan_session_reconnect(reconnect_source.clone(), 0, false)
        .unwrap();
    assert_eq!(immediate.action, "retry");
    assert_eq!(immediate.delay_millis, Some(0));
    assert!(immediate.restore_last_context);
    assert_eq!(
        bridge
            .plan_session_reconnect(reconnect_source.clone(), 6, false)
            .unwrap()
            .delay_millis,
        Some(30_000)
    );
    assert_eq!(
        bridge
            .plan_session_reconnect(reconnect_source.clone(), 7, false)
            .unwrap()
            .action,
        "exhausted"
    );
    assert_eq!(
        bridge
            .plan_session_reconnect(reconnect_source.clone(), 0, true)
            .unwrap()
            .action,
        "authentication_stopped"
    );
    let prompt_stop = bridge
        .reconnect_saved_session(reconnect_source.clone(), None)
        .unwrap();
    assert_eq!(prompt_stop.state, "authentication_stopped");
    assert_eq!(prompt_stop.session_id, None);
    let retryable = bridge
        .reconnect_saved_session(reconnect_source.clone(), Some("unused".into()))
        .unwrap();
    assert_eq!(retryable.state, "retryable");
    assert_eq!(retryable.session_id, None);
    assert!(
        bridge
            .list_profiles()
            .unwrap()
            .iter()
            .find(|item| item.id_bytes == profile_id.to_bytes())
            .unwrap()
            .connected
    );
    let history_operation = execute(
        &bridge,
        reconnect_source.clone(),
        history_result_id,
        "SELECT history_full",
    );
    bridge.pump(history_operation).unwrap();
    let history = bridge
        .list_history(Some("history_full".into()), 10)
        .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].engine, "postgresql");
    assert_eq!(history[0].database_name, "postgres");
    assert_eq!(
        history[0].statement_text.as_deref(),
        Some("SELECT history_full")
    );
    assert_eq!(history[0].outcome, "completed");

    bridge
        .set_history_retention("metadata_only".into())
        .unwrap();
    assert_eq!(bridge.history_retention().unwrap(), "metadata_only");
    let (metadata_result_id, metadata_page) = sample_page(Engine::PostgreSql, 143, &[2]);
    let metadata_session = bridge
        .open_driver_session_for_profile(
            profile_id,
            Engine::PostgreSql,
            Box::new(FixedPageSession {
                engine: Engine::PostgreSql,
                page: metadata_page,
                health_failure: None,
            }),
        )
        .unwrap();
    let metadata_operation = execute(
        &bridge,
        metadata_session.clone(),
        metadata_result_id,
        "SELECT history_metadata",
    );
    bridge.pump(metadata_operation).unwrap();
    bridge.disconnect(metadata_session).unwrap();
    let history = bridge.list_history(None, 10).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].statement_text, None);

    bridge.set_history_retention("private".into()).unwrap();
    assert_eq!(bridge.history_retention().unwrap(), "private");
    let (private_result_id, private_page) = sample_page(Engine::PostgreSql, 144, &[3]);
    let private_session = bridge
        .open_driver_session_for_profile(
            profile_id,
            Engine::PostgreSql,
            Box::new(FixedPageSession {
                engine: Engine::PostgreSql,
                page: private_page,
                health_failure: None,
            }),
        )
        .unwrap();
    let private_operation = execute(
        &bridge,
        private_session.clone(),
        private_result_id,
        "SELECT history_private",
    );
    bridge.pump(private_operation).unwrap();
    bridge.disconnect(private_session).unwrap();
    assert_eq!(bridge.list_history(None, 10).unwrap().len(), 2);
    let saved_id = bridge
        .save_query(
            "Recent users".into(),
            "postgresql".into(),
            "SELECT * FROM users".into(),
        )
        .unwrap();
    let redis_id = bridge
        .save_query("Redis scan".into(), "redis".into(), "SCAN 0".into())
        .unwrap();
    assert_ne!(saved_id, redis_id);
    let postgres_saved = bridge
        .list_saved_queries(Some("postgresql".into()), Some("users".into()))
        .unwrap();
    assert_eq!(postgres_saved.len(), 1);
    assert_eq!(postgres_saved[0].query_id, saved_id);
    assert_eq!(postgres_saved[0].name, "Recent users");
    assert_eq!(postgres_saved[0].statement_text, "SELECT * FROM users");
    assert_eq!(
        bridge
            .save_query(
                "Recent users".into(),
                "postgresql".into(),
                "SELECT id FROM users".into(),
            )
            .unwrap(),
        saved_id
    );
    assert_eq!(
        bridge
            .list_saved_queries(None, Some("id from".into()))
            .unwrap()[0]
            .statement_text,
        "SELECT id FROM users"
    );
    assert!(bridge.delete_saved_query(saved_id).unwrap());
    assert_eq!(bridge.list_saved_queries(None, None).unwrap().len(), 1);
    assert!(bridge.delete_saved_query(redis_id).unwrap());
    let intent = BridgeSessionIntent {
        database: "postgres".into(),
        schema: Some("public".into()),
        selected_tab: 1,
        tabs: vec![
            BridgeWorkspaceTab {
                title: "Query 1".into(),
                statement_text: "SELECT 1;".into(),
            },
            BridgeWorkspaceTab {
                title: "Users".into(),
                statement_text: "SELECT id FROM users;".into(),
            },
        ],
    };
    bridge
        .put_session_intent(profile_id.to_bytes().to_vec(), intent.clone())
        .unwrap();
    assert_eq!(
        bridge
            .get_session_intent(profile_id.to_bytes().to_vec())
            .unwrap(),
        Some(intent)
    );
    bridge
        .delete_session_intent(profile_id.to_bytes().to_vec())
        .unwrap();
    assert_eq!(
        bridge
            .get_session_intent(profile_id.to_bytes().to_vec())
            .unwrap(),
        None
    );
    let window_one = "11111111-1111-4111-8111-111111111111";
    let window_two = "22222222-2222-4222-8222-222222222222";
    let first_intent = BridgeSessionIntent {
        database: "postgres".into(),
        schema: Some("public".into()),
        selected_tab: 0,
        tabs: vec![BridgeWorkspaceTab {
            title: "First window".into(),
            statement_text: "SELECT 1;".into(),
        }],
    };
    let second_intent = BridgeSessionIntent {
        database: "postgres".into(),
        schema: Some("audit".into()),
        selected_tab: 0,
        tabs: vec![BridgeWorkspaceTab {
            title: "Second window".into(),
            statement_text: "SELECT 2;".into(),
        }],
    };
    bridge
        .put_native_window_intent(
            window_one.into(),
            profile_id.to_bytes().to_vec(),
            first_intent.clone(),
        )
        .unwrap();
    bridge
        .put_native_window_intent(
            window_two.into(),
            profile_id.to_bytes().to_vec(),
            second_intent.clone(),
        )
        .unwrap();
    let restored_one = bridge
        .get_native_window_intent(window_one.into())
        .unwrap()
        .unwrap();
    let restored_two = bridge
        .get_native_window_intent(window_two.into())
        .unwrap()
        .unwrap();
    assert_eq!(restored_one.profile_id, profile_id.to_bytes());
    assert_eq!(restored_one.intent, first_intent);
    assert_eq!(restored_two.intent, second_intent);
    bridge
        .delete_native_window_intent(window_one.into())
        .unwrap();
    assert!(
        bridge
            .get_native_window_intent(window_one.into())
            .unwrap()
            .is_none()
    );
    assert!(
        bridge
            .get_native_window_intent(window_two.into())
            .unwrap()
            .is_some()
    );
    bridge.disconnect(reconnect_source).unwrap();
    assert_eq!(
        bridge
            .search_profiles(Some("POSTGRES".into()))
            .unwrap()
            .len(),
        1
    );
    assert!(
        bridge
            .search_profiles(Some("missing".into()))
            .unwrap()
            .is_empty()
    );
    let mut edited = bridge
        .get_profile_draft(profile_id.to_bytes().to_vec())
        .unwrap();
    assert_eq!(edited.revision, 0);
    assert_eq!(edited.password_source, "prompt");
    assert!(!edited.has_stored_password);
    edited.name = "bridge-edited".into();
    assert_eq!(
        bridge.save_profile(edited.clone()).unwrap(),
        profile_id.to_bytes()
    );
    let replacement = bridge
        .get_profile_draft(profile_id.to_bytes().to_vec())
        .unwrap();
    assert_eq!(replacement.revision, 1);
    assert_eq!(replacement.name, "bridge-edited");

    edited.id_bytes = None;
    edited.revision = 0;
    edited.name = "bridge-copy".into();
    let copy_id = bridge.save_profile(edited).unwrap();
    assert_ne!(copy_id, profile_id.to_bytes());
    assert_eq!(bridge.list_profiles().unwrap().len(), 2);
    let copy_profile_id = ProfileId::from_bytes(copy_id.clone().try_into().unwrap()).unwrap();
    let (copy_result_id, copy_page) = sample_page(Engine::PostgreSql, 142, &[1]);
    let retained_session = bridge
        .open_driver_session_for_profile(
            copy_profile_id,
            Engine::PostgreSql,
            Box::new(FixedPageSession {
                engine: Engine::PostgreSql,
                page: copy_page,
                health_failure: None,
            }),
        )
        .unwrap();
    assert!(
        bridge
            .list_profiles()
            .unwrap()
            .iter()
            .find(|item| item.id_bytes == copy_id)
            .unwrap()
            .connected
    );
    bridge.delete_profile(copy_id, 0).unwrap();
    assert_eq!(bridge.list_profiles().unwrap().len(), 1);
    let retained_operation = probe(&bridge, retained_session.clone(), copy_result_id);
    bridge.pump(retained_operation).unwrap();
    bridge.disconnect(retained_session).unwrap();
    bridge
        .set_profile_favorite(profile_id.to_bytes().to_vec(), 1, true)
        .unwrap();
    let favorite = bridge.list_profiles().unwrap().remove(0);
    assert!(favorite.favorite);
    assert_eq!(favorite.revision, 2);
    bridge
        .reorder_profiles(
            Some("g".into()),
            vec![BridgeProfileOrderItem {
                id_bytes: favorite.id_bytes,
                expected_revision: favorite.revision,
            }],
        )
        .unwrap();
    assert_eq!(bridge.list_profiles().unwrap()[0].revision, 3);
    bridge.create_profile_group("Empty".into()).unwrap();
    let groups = bridge.list_profile_groups().unwrap();
    assert_eq!(
        groups
            .iter()
            .map(|group| group.name.as_str())
            .collect::<Vec<_>>(),
        ["Empty", "g"]
    );
    assert!(!groups[0].alphabetical);
    bridge
        .set_profile_group_alphabetical("Empty".into(), true)
        .unwrap();
    assert!(bridge.list_profile_groups().unwrap()[0].alphabetical);
    assert_eq!(
        bridge
            .rename_profile_group("Empty".into(), "Renamed".into())
            .unwrap(),
        0
    );
    assert_eq!(
        bridge
            .list_profile_groups()
            .unwrap()
            .iter()
            .map(|group| group.name.as_str())
            .collect::<Vec<_>>(),
        ["Renamed", "g"]
    );
    assert_eq!(bridge.delete_profile_group("Renamed".into()).unwrap(), 0);
    let prompt = bridge
        .open_profile(profile_id.to_bytes().to_vec(), None)
        .unwrap_err();
    assert!(matches!(
        prompt,
        BridgeError::Rejected { ref code, .. } if code == "profile-password"
    ));
    // Port 1 is unreachable — proves load+connect path without a live server.
    let err = bridge
        .open_profile(profile_id.to_bytes().to_vec(), Some("unused".into()))
        .unwrap_err();
    assert!(matches!(
        err,
        BridgeError::Rejected { ref code, .. } if code == "connect"
    ));
    let mut copy = bridge
        .get_profile_draft(profile_id.to_bytes().to_vec())
        .unwrap();
    copy.id_bytes = None;
    copy.revision = 0;
    for index in 0..100 {
        copy.name = format!("paged-{index:03}");
        bridge.save_profile(copy.clone()).unwrap();
    }
    assert_eq!(bridge.list_profiles().unwrap().len(), 101);
    assert_eq!(
        bridge
            .search_profiles(Some("paged-099".into()))
            .unwrap()
            .len(),
        1
    );
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
fn sql_file_bridge_is_atomic_and_blocks_external_overwrite() {
    let bridge = TableRockBridge::new_for_test();
    let path = std::env::temp_dir().join(format!(
        "tablerock-ffi-sql-{}-{}.sql",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let raw = path.to_string_lossy().into_owned();
    let first = bridge
        .write_sql_file(raw.clone(), "SELECT 1;\n".into(), None, None, false)
        .unwrap();
    assert_eq!(bridge.read_sql_file(raw.clone()).unwrap(), first);

    std::fs::write(&path, "SELECT 22;\n").unwrap();
    let error = bridge
        .write_sql_file(
            raw.clone(),
            "SELECT 3;\n".into(),
            first.modified_nanos,
            Some(first.len),
            false,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        BridgeError::Rejected { ref code, .. } if code == "sql-file-external-change"
    ));
    let overwritten = bridge
        .write_sql_file(
            raw.clone(),
            "SELECT 3;\n".into(),
            first.modified_nanos,
            Some(first.len),
            true,
        )
        .unwrap();
    assert_eq!(overwritten.statement_text, "SELECT 3;\n");
    let _ = std::fs::remove_file(path);
}

#[test]
fn session_health_projects_authentication_as_terminal_state() {
    let (_, page) = sample_page(Engine::PostgreSql, 84, &[1]);
    let bridge = TableRockBridge::new_for_test();
    let session = bridge
        .open_driver_session(
            Engine::PostgreSql,
            Box::new(FixedPageSession {
                engine: Engine::PostgreSql,
                page,
                health_failure: Some(AdapterFailureClass::Authentication),
            }),
        )
        .unwrap();
    let health = bridge.check_session_health(session.clone()).unwrap();
    assert_eq!(health.state, "authentication_stopped");
    assert!(!health.server_reachable);
    assert_eq!(health.elapsed_millis, None);
    assert!(health.authentication_stopped);
    bridge.disconnect(session).unwrap();
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
