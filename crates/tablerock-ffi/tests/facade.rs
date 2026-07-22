//! Facade unit tests: runtime lifecycle, panic containment, page encode path.

use std::{
    fs,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use tablerock_core::{
    BoundedText, ByteLimit, CancelDispatch, ColumnMetadata, Engine, EngineType, IdParts,
    OperationId, OwnedValue, PageDelivery, PageFacts, PageIdentity, PageLimits, PageWarnings,
    ResultId, ResultPage, Revision, RowTotal,
};
use tablerock_engine::{
    AdapterError, AdapterFailureClass, DriverFuture, DriverPageRequest, DriverPageStream,
    DriverSession, PostgresActivityRow, ServerDescribe, SessionHealth,
};
use tablerock_ffi::{BridgeError, SubmitSpec, TableRockBridge};

struct OnePageStream {
    page: Option<ResultPage>,
    fail: bool,
    hold: bool,
}

struct ActivitySession {
    signals: Arc<Mutex<Vec<(bool, i32)>>>,
}

impl DriverSession for ActivitySession {
    fn engine(&self) -> Engine {
        Engine::PostgreSql
    }

    fn start_page_stream<'a>(
        &'a self,
        _request: DriverPageRequest,
    ) -> DriverFuture<'a, Result<Box<dyn DriverPageStream>, AdapterError>> {
        Box::pin(async {
            Err(AdapterError::new(
                Engine::PostgreSql,
                AdapterFailureClass::InvalidRequest,
            ))
        })
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

    fn postgres_activity<'a>(
        &'a self,
    ) -> DriverFuture<'a, Result<Vec<PostgresActivityRow>, AdapterError>> {
        Box::pin(async {
            Ok(vec![PostgresActivityRow::new(
                42,
                "fixture".into(),
                "TableRock".into(),
                "active".into(),
                "SELECT bounded preview".into(),
            )])
        })
    }

    fn signal_postgres_backend<'a>(
        &'a self,
        terminate: bool,
        pid: i32,
    ) -> DriverFuture<'a, Result<bool, AdapterError>> {
        self.signals.lock().unwrap().push((terminate, pid));
        Box::pin(async { Ok(true) })
    }

    fn shutdown(self: Box<Self>) -> DriverFuture<'static, Result<(), AdapterError>> {
        Box::pin(async { Ok(()) })
    }
}

impl DriverPageStream for OnePageStream {
    fn next_page<'a>(
        &'a mut self,
        _identity: PageIdentity,
        _start_row: u64,
    ) -> DriverFuture<'a, Result<Option<ResultPage>, AdapterError>> {
        Box::pin(async move {
            if self.hold {
                std::future::pending::<()>().await;
            }
            if self.fail {
                return Err(AdapterError::new(
                    Engine::PostgreSql,
                    AdapterFailureClass::Query,
                ));
            }
            Ok(self.page.take())
        })
    }
}

struct FixedPageSession {
    page: ResultPage,
    fail: bool,
    hold: bool,
    shutdown: Arc<AtomicBool>,
    expected_statement: Option<&'static str>,
}

impl DriverSession for FixedPageSession {
    fn engine(&self) -> Engine {
        Engine::PostgreSql
    }

    fn start_page_stream<'a>(
        &'a self,
        request: DriverPageRequest,
    ) -> DriverFuture<'a, Result<Box<dyn DriverPageStream>, AdapterError>> {
        if let Some(expected) = self.expected_statement {
            match request {
                DriverPageRequest::PostgreSqlStatement { statement, .. } => {
                    assert_eq!(statement.as_str(), expected);
                }
                other => panic!("expected PostgreSQL statement request, got {other:?}"),
            }
        }
        let page = self.page.clone();
        let fail = self.fail;
        let hold = self.hold;
        Box::pin(async move {
            Ok(Box::new(OnePageStream {
                page: Some(page),
                fail,
                hold,
            }) as Box<dyn DriverPageStream>)
        })
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
fn connection_url_becomes_unsaved_review_draft() {
    let bridge = TableRockBridge::new_for_test();
    let draft = bridge
        .parse_connection_url_draft(
            "postgresql://fixture:secret@db.example:5433/app?sslmode=require".into(),
        )
        .unwrap();

    assert_eq!(draft.engine, "postgresql");
    assert_eq!(draft.host, "db.example");
    assert_eq!(draft.port, "5433");
    assert_eq!(draft.database, "app");
    assert_eq!(draft.username, "fixture");
    assert_eq!(draft.password_source, "keychain");
    assert_eq!(draft.password_value, "secret");
    assert_eq!(draft.tls_mode, "verify_full");
    assert!(draft.id_bytes.is_none());

    let error = bridge
        .parse_connection_url_draft("javascript://example.invalid".into())
        .unwrap_err();
    assert!(matches!(error, BridgeError::Rejected { code, .. } if code == "connection-url"));
}

#[test]
fn postgres_activity_and_signals_use_typed_driver_contract() {
    let bridge = TableRockBridge::new_for_test();
    let signals = Arc::new(Mutex::new(Vec::new()));
    let session = bridge
        .open_driver_session(
            Engine::PostgreSql,
            Box::new(ActivitySession {
                signals: Arc::clone(&signals),
            }),
        )
        .unwrap();

    let rows = bridge.postgres_activity(session.clone()).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].pid, 42);
    assert_eq!(rows[0].user, "fixture");
    assert_eq!(rows[0].query_preview, "SELECT bounded preview");

    let cancel = bridge
        .signal_postgres_backend(session.clone(), "cancel".into(), 42)
        .unwrap();
    assert_eq!(cancel.kind, "cancel");
    assert!(cancel.acknowledged);
    bridge
        .signal_postgres_backend(session.clone(), "terminate".into(), 43)
        .unwrap();
    assert_eq!(*signals.lock().unwrap(), vec![(false, 42), (true, 43)]);

    let invalid = bridge
        .signal_postgres_backend(session, "kill".into(), 42)
        .unwrap_err();
    assert!(
        matches!(invalid, BridgeError::Rejected { code, .. } if code == "postgres-activity-signal")
    );
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
                fail: false,
                hold: false,
                shutdown: Arc::new(AtomicBool::new(false)),
                expected_statement: None,
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

    let decoded =
        ResultPage::decode_v1(&fetched, PageLimits::new(500, 64, 1024 * 1024, 64 * 1024)).unwrap();
    assert_eq!(decoded.cell(0, 0).unwrap().bytes(), &42_i64.to_be_bytes());

    let shutdown = bridge.shutdown(false, 1_000).unwrap();
    assert!(shutdown.active_operations == 0 || !shutdown.core.is_empty());
}

#[test]
fn explain_intent_builds_safe_postgresql_statement() {
    let result_id = ResultId::from_parts(IdParts::new(0, 199).unwrap()).unwrap();
    let bridge = TableRockBridge::new_for_test();
    let session_id = bridge
        .open_driver_session(
            Engine::PostgreSql,
            Box::new(FixedPageSession {
                page: sample_page(result_id),
                fail: false,
                hold: false,
                shutdown: Arc::new(AtomicBool::new(false)),
                expected_statement: Some("EXPLAIN (FORMAT TEXT) explainable_table"),
            }),
        )
        .unwrap();

    let operation_id = bridge
        .submit(SubmitSpec {
            intent: "explain".into(),
            session_id,
            statement: Some("explainable_table".into()),
            result_id: Some(result_id.to_bytes().to_vec()),
            start_row: None,
            row_count: Some(100),
            expected_revision: 0,
        })
        .unwrap();
    bridge.pump(operation_id).unwrap();
}

#[test]
fn failed_runtime_outcome_enters_safe_support_bundle() {
    let result_id = ResultId::from_parts(IdParts::new(0, 100).unwrap()).unwrap();
    let bridge = TableRockBridge::new_for_test();
    let session_id = bridge
        .open_driver_session(
            Engine::PostgreSql,
            Box::new(FixedPageSession {
                page: sample_page(result_id),
                fail: true,
                hold: false,
                shutdown: Arc::new(AtomicBool::new(false)),
                expected_statement: None,
            }),
        )
        .unwrap();
    let operation_id = bridge
        .submit(SubmitSpec {
            intent: "probe".into(),
            session_id,
            statement: Some("secret statement".into()),
            result_id: Some(result_id.to_bytes().to_vec()),
            start_row: None,
            row_count: Some(100),
            expected_revision: 0,
        })
        .unwrap();
    bridge.pump(operation_id).unwrap();

    let directory =
        std::env::temp_dir().join(format!("tablerock-support-runtime-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let destination = directory.join("support.txt");
    bridge
        .export_support_bundle(destination.to_string_lossy().into_owned())
        .unwrap();
    let payload = fs::read_to_string(&destination).unwrap();
    assert!(payload.contains("diagnostics.count=1\n"));
    assert!(payload.contains("diagnostic.0=PostgreSql|Server|None|Error"));
    assert!(payload.contains("operation_outcome.0=PostgreSql|Failed\n"));
    assert!(!payload.contains("secret statement"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn cancel_active_shutdown_drains_within_deadline() {
    let result_id = ResultId::from_parts(IdParts::new(0, 101).unwrap()).unwrap();
    let bridge = TableRockBridge::new_for_test();
    let session_id = bridge
        .open_driver_session(
            Engine::PostgreSql,
            Box::new(FixedPageSession {
                page: sample_page(result_id),
                fail: false,
                hold: true,
                shutdown: Arc::new(AtomicBool::new(false)),
                expected_statement: None,
            }),
        )
        .unwrap();
    bridge
        .submit(SubmitSpec {
            intent: "probe".into(),
            session_id,
            statement: Some("select pg_sleep(10)".into()),
            result_id: Some(result_id.to_bytes().to_vec()),
            start_row: None,
            row_count: Some(100),
            expected_revision: 0,
        })
        .unwrap();

    let started = std::time::Instant::now();
    let outcome = bridge.shutdown(true, 1_000).unwrap();
    assert_eq!(outcome.active_operations, 0);
    assert_eq!(outcome.core, "Stopped");
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn graceful_shutdown_reports_active_work_at_deadline() {
    let result_id = ResultId::from_parts(IdParts::new(0, 102).unwrap()).unwrap();
    let bridge = TableRockBridge::new_for_test();
    let session_id = bridge
        .open_driver_session(
            Engine::PostgreSql,
            Box::new(FixedPageSession {
                page: sample_page(result_id),
                fail: false,
                hold: true,
                shutdown: Arc::new(AtomicBool::new(false)),
                expected_statement: None,
            }),
        )
        .unwrap();
    bridge
        .submit(SubmitSpec {
            intent: "probe".into(),
            session_id,
            statement: Some("select pg_sleep(10)".into()),
            result_id: Some(result_id.to_bytes().to_vec()),
            start_row: None,
            row_count: Some(100),
            expected_revision: 0,
        })
        .unwrap();

    let started = std::time::Instant::now();
    let outcome = bridge.shutdown(false, 20).unwrap();
    assert_eq!(outcome.active_operations, 1);
    assert_eq!(outcome.core, "Draining { active_operations: 1 }");
    assert!(started.elapsed() < Duration::from_millis(500));

    let drained = bridge.shutdown(true, 1_000).unwrap();
    assert_eq!(drained.active_operations, 0);
    assert_eq!(drained.core, "Stopped");
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
        tls_mode: "off".into(),
    };
    let debug = format!("{params:?}");
    assert!(!debug.contains("super-secret"));
    assert!(debug.contains("<redacted>"));
}

#[test]
fn cancel_unknown_operation_is_typed() {
    let bridge = TableRockBridge::new_for_test();
    bridge.ensure_runtime().unwrap();
    let bogus = ResultId::from_parts(IdParts::new(0, 3).unwrap())
        .unwrap()
        .to_bytes()
        .to_vec();
    // Operation id namespace shares 16-byte layout; unknown op returns typed outcome.
    let outcome = bridge.cancel(bogus).unwrap();
    assert!(
        outcome.core.contains("Unknown") || outcome.core.contains("unknown"),
        "core={}",
        outcome.core
    );
}

#[test]
fn open_rejects_unreachable_endpoint() {
    let bridge = TableRockBridge::new_for_test();
    for (engine, database, user) in [
        ("postgresql", "postgres", "postgres"),
        ("clickhouse", "default", "default"),
        ("redis", "0", ""),
    ] {
        let err = bridge
            .open(tablerock_ffi::OpenParams {
                engine: engine.into(),
                host: "127.0.0.1".into(),
                port: 1,
                database: database.into(),
                user: user.into(),
                password: String::new(),
                tls_mode: "off".into(),
            })
            .unwrap_err();
        match err {
            BridgeError::Rejected { code, .. } => assert_eq!(code, "connect", "{engine}"),
            other => panic!("expected {engine} connect reject, got {other:?}"),
        }
    }
}

#[test]
fn support_bundle_export_is_atomic_safe_schema() {
    let bridge = TableRockBridge::new_for_test();
    let directory = std::env::temp_dir().join(format!(
        "tablerock-support-export-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    fs::create_dir_all(&directory).unwrap();
    let destination = directory.join("support.txt");

    let bytes = bridge
        .export_support_bundle(destination.to_string_lossy().into_owned())
        .unwrap();
    let payload = fs::read_to_string(&destination).unwrap();
    assert_eq!(bytes as usize, payload.len());
    assert!(payload.starts_with("schema=2\nclient.version="));
    assert!(payload.contains("diagnostics.count=0\n"));
    assert!(payload.contains("operation_outcomes.count=0\n"));
    for forbidden in ["password", "SELECT", "localhost", "cell-value"] {
        assert!(!payload.contains(forbidden));
    }
    fs::remove_dir_all(directory).unwrap();
}
