//! Real-server bridge path: open → submit probe → pump → fetch_page → shutdown
//! through the synchronous UniFFI facade against Docker engines.
//!
//! The facade owns a multi-thread Tokio runtime and uses `block_on`. Tests start
//! containers on the async test runtime, then call the bridge from
//! `spawn_blocking` so runtimes never nest.

use tablerock_core::{Engine, PageLimits, ResultPage};
use tablerock_ffi::{
    BridgeCsvImportRequest, BridgeDdlChangeRequest, BridgePostgresToolRequest,
    BridgeQueryParameter, BridgeStartupActionDraft, BridgeStreamExportRequest,
    BridgeTableOperationRequest, OpenParams, SubmitSpec, TableRockBridge,
};
use testcontainers::{
    GenericImage, ImageExt,
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};

fn pg_dump_explicit_path() -> Option<String> {
    [
        "/opt/homebrew/opt/libpq/bin/pg_dump",
        "/usr/local/opt/libpq/bin/pg_dump",
        "/usr/bin/pg_dump",
    ]
    .into_iter()
    .find(|path| std::path::Path::new(path).is_file())
    .map(str::to_owned)
}

fn open_params(engine: &str, host: &str, port: u16, database: &str, user: &str) -> OpenParams {
    OpenParams {
        engine: engine.into(),
        host: host.into(),
        port,
        database: database.into(),
        user: user.into(),
        password: String::new(),
        tls_mode: "off".into(),
    }
}

fn open_when_ready(
    bridge: &TableRockBridge,
    engine: &str,
    host: &str,
    port: u16,
    database: &str,
    user: &str,
) -> Vec<u8> {
    let mut last_err = None;
    for attempt in 0..40 {
        match bridge.open(open_params(engine, host, port, database, user)) {
            Ok(session) => return session,
            Err(error) => {
                last_err = Some(error.to_string());
                if attempt < 39 {
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
            }
        }
    }
    panic!("{engine} did not become query-ready: {last_err:?}");
}

/// Returns (page_bytes, next_event_cursor).
fn probe_and_fetch(
    bridge: &TableRockBridge,
    session_id: Vec<u8>,
    event_cursor: u64,
) -> (Vec<u8>, u64) {
    let operation = bridge
        .submit(SubmitSpec {
            intent: "probe".into(),
            session_id,
            statement: Some("select 1".into()),
            result_id: None,
            start_row: None,
            row_count: Some(64),
            expected_revision: 0,
        })
        .expect("submit probe");
    bridge.pump(operation.clone()).expect("pump to terminal");
    let batch = bridge.next_events(event_cursor, 64).expect("events");
    assert!(
        batch.events.iter().any(|e| e.kind == "page"),
        "expected page event after cursor {event_cursor}, got {:?} outcomes {:?}",
        batch
            .events
            .iter()
            .map(|e| e.kind.as_str())
            .collect::<Vec<_>>(),
        batch
            .events
            .iter()
            .filter(|e| e.kind == "terminal")
            .map(|e| e.outcome.clone())
            .collect::<Vec<_>>(),
    );
    let page_bytes = batch
        .events
        .iter()
        .rev()
        .find(|e| e.kind == "page")
        .and_then(|e| e.page_bytes.clone())
        .expect("page bytes on event");
    assert!(page_bytes.starts_with(b"TRP1"));
    let decoded = ResultPage::decode_v1(
        &page_bytes,
        PageLimits::new(500, 64, 4 * 1024 * 1024, 64 * 1024),
    )
    .expect("decode page");
    let result_id = decoded.envelope().result_id().to_bytes().to_vec();
    let fetched = bridge
        .fetch_page(result_id, 0, decoded.envelope().revision().get())
        .expect("fetch_page");
    assert_eq!(fetched, page_bytes);
    (page_bytes, batch.next_cursor)
}

fn execute(bridge: &TableRockBridge, session_id: Vec<u8>, statement: &str) -> Vec<u8> {
    let operation = bridge
        .submit(SubmitSpec {
            intent: "execute".into(),
            session_id,
            statement: Some(statement.into()),
            result_id: None,
            start_row: None,
            row_count: Some(64),
            expected_revision: 0,
        })
        .unwrap();
    bridge.pump(operation.clone()).unwrap();
    bridge
        .next_events(0, 256)
        .unwrap()
        .events
        .into_iter()
        .find(|event| event.operation_id == operation && event.kind == "page")
        .and_then(|event| event.page_bytes)
        .unwrap_or_default()
}

fn wait_csv_import(
    bridge: &TableRockBridge,
    operation_id: Vec<u8>,
) -> tablerock_ffi::BridgeCsvImportProgress {
    (0..30_000)
        .find_map(|_| {
            let progress = bridge.csv_import_progress(operation_id.clone()).unwrap();
            if progress.phase == "running" || progress.phase == "cancel_requested" {
                std::thread::sleep(std::time::Duration::from_millis(1));
                None
            } else {
                Some(progress)
            }
        })
        .expect("CSV import reaches a terminal state")
}

fn wait_stream_export(
    bridge: &TableRockBridge,
    operation_id: Vec<u8>,
) -> tablerock_ffi::BridgeStreamExportProgress {
    (0..30_000)
        .find_map(|_| {
            let progress = bridge.stream_export_progress(operation_id.clone()).unwrap();
            if progress.phase == "running" || progress.phase == "cancel_requested" {
                std::thread::sleep(std::time::Duration::from_millis(1));
                None
            } else {
                Some(progress)
            }
        })
        .expect("stream export reaches a terminal state")
}

#[ignore = "real-server test: runs in CI real-servers job with --include-ignored"]
#[tokio::test]
async fn bridge_postgres_open_probe_fetch_shutdown() {
    let container = GenericImage::new("postgres", "18.4-alpine")
        .with_exposed_port(5432.tcp())
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_HOST_AUTH_METHOD", "trust")
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(5432.tcp()).await.unwrap();
    let host = container.get_host().await.unwrap().to_string();

    tokio::task::spawn_blocking(move || {
        let bridge = TableRockBridge::new_for_test();
        let profile_path = std::env::temp_dir().join(format!(
            "tablerock-native-startup-{}-{}.db",
            std::process::id(),
            port
        ));
        bridge
            .configure_persistence(profile_path.to_string_lossy().into_owned())
            .unwrap();
        let mut startup_profile = bridge
            .parse_connection_url_draft(format!("postgresql://postgres@{}:{port}/postgres", host))
            .unwrap();
        startup_profile.name = "startup-live".into();
        startup_profile.startup_actions = vec![
            BridgeStartupActionDraft {
                statement: "SET application_name = 'tablerock_startup_live'".into(),
                safety: "read_only".into(),
                timeout_ms: 5_000,
                run_on_reconnect: true,
            },
            BridgeStartupActionDraft {
                statement: "CREATE TABLE startup_must_not_run(id integer)".into(),
                safety: "write".into(),
                timeout_ms: 5_000,
                run_on_reconnect: true,
            },
        ];
        let profile_id = bridge.save_profile(startup_profile).unwrap();
        let startup_session = bridge
            .open_profile_with_secret(profile_id, Some(b"unused".to_vec()))
            .unwrap();
        let startup_page = execute(
            &bridge,
            startup_session.clone(),
            "SELECT current_setting('application_name'), to_regclass('startup_must_not_run')",
        );
        let startup_page = ResultPage::decode_v1(
            &startup_page,
            PageLimits::new(500, 64, 4 * 1024 * 1024, 64 * 1024),
        )
        .unwrap();
        assert_eq!(
            startup_page.cell(0, 0).unwrap().bytes(),
            b"tablerock_startup_live"
        );
        assert!(startup_page.cell(0, 1).unwrap().is_null());
        bridge.disconnect(startup_session).unwrap();
        let _ = std::fs::remove_file(&profile_path);
        let _ = std::fs::remove_file(format!("{}-wal", profile_path.display()));
        let _ = std::fs::remove_file(format!("{}-shm", profile_path.display()));
        let session = open_when_ready(&bridge, "postgresql", &host, port, "postgres", "postgres");
        let activity = bridge.postgres_activity(session.clone()).unwrap();
        assert!(!activity.is_empty());
        assert!(activity.iter().all(|row| row.pid > 0));
        let hostile_value = "42' OR TRUE --";
        let named = bridge
            .submit_named(
                SubmitSpec {
                    intent: "execute".into(),
                    session_id: session.clone(),
                    statement: Some("SELECT :value::text AS bound_value".into()),
                    result_id: None,
                    start_row: None,
                    row_count: Some(16),
                    expected_revision: 0,
                },
                vec![BridgeQueryParameter {
                    name: "value".into(),
                    kind: "text".into(),
                    value: Some(hostile_value.into()),
                }],
            )
            .unwrap();
        bridge.pump(named.clone()).unwrap();
        let named_page = bridge
            .next_events(0, 64)
            .unwrap()
            .events
            .into_iter()
            .find(|event| event.operation_id == named && event.kind == "page")
            .and_then(|event| event.page_bytes)
            .unwrap();
        let named_page = ResultPage::decode_v1(
            &named_page,
            PageLimits::new(500, 64, 4 * 1024 * 1024, 64 * 1024),
        )
        .unwrap();
        assert_eq!(
            named_page.cell(0, 0).unwrap().bytes(),
            hostile_value.as_bytes()
        );
        let create = bridge
            .submit(SubmitSpec {
                intent: "execute".into(),
                session_id: session.clone(),
                statement: Some("CREATE TABLE IF NOT EXISTS bridge_ddl_review (id integer)".into()),
                result_id: None,
                start_row: None,
                row_count: Some(16),
                expected_revision: 0,
            })
            .unwrap();
        bridge.pump(create).unwrap();
        execute(
            &bridge,
            session.clone(),
            "CREATE TABLE IF NOT EXISTS bridge_stream_import (id bigint, name text)",
        );
        let database = bridge
            .refresh_catalog(session.clone(), None)
            .unwrap()
            .into_iter()
            .find(|node| node.name == "postgres")
            .unwrap();
        let schema = bridge
            .refresh_catalog(session.clone(), Some(database.id_bytes))
            .unwrap()
            .into_iter()
            .find(|node| node.name == "public")
            .unwrap();
        let relation = bridge
            .refresh_catalog(session.clone(), Some(schema.id_bytes.clone()))
            .unwrap()
            .into_iter()
            .find(|node| node.name == "bridge_ddl_review")
            .unwrap();
        let review = bridge
            .stage_ddl_change(BridgeDdlChangeRequest {
                session_id: session.clone(),
                catalog_node_id: relation.id_bytes.clone(),
                kind: "add_column".into(),
                object_name: "reviewed_name".into(),
                definition: "text".into(),
                now_ms: 1_000,
            })
            .unwrap();
        assert_eq!(
            review.preview,
            "ALTER TABLE \"public\".\"bridge_ddl_review\" ADD COLUMN \"reviewed_name\" text;"
        );
        assert!(!review.destructive);
        bridge
            .apply_ddl_change(review.token_id.clone(), session.clone(), 2_000, true)
            .unwrap();
        assert!(
            bridge
                .relation_structure(session.clone(), relation.id_bytes.clone())
                .unwrap()
                .columns
                .iter()
                .any(|column| column.name == "reviewed_name")
        );
        let consumed = bridge
            .apply_ddl_change(review.token_id, session.clone(), 2_001, true)
            .unwrap_err();
        assert!(consumed.to_string().contains("missing or consumed"));
        let refreshed_relations = bridge
            .refresh_catalog(session.clone(), Some(schema.id_bytes.clone()))
            .unwrap();
        let import_relation = refreshed_relations
            .iter()
            .find(|node| node.name == "bridge_stream_import")
            .unwrap()
            .clone();
        let relation = refreshed_relations
            .into_iter()
            .find(|node| node.name == "bridge_ddl_review")
            .unwrap();
        let import_path = std::env::temp_dir().join(format!(
            "tablerock-pg-stream-import-{}.csv",
            std::process::id()
        ));
        let mut import_csv = String::from("id,name\n");
        for row in 0..1_200_u64 {
            import_csv.push_str(&format!("{row},row-{row}\n"));
        }
        std::fs::write(&import_path, import_csv).unwrap();
        let import_preview = bridge
            .preview_csv_import(import_path.to_string_lossy().into_owned())
            .unwrap();
        let import_review = bridge
            .stage_csv_import(BridgeCsvImportRequest {
                session_id: session.clone(),
                catalog_node_id: import_relation.id_bytes,
                path: import_path.to_string_lossy().into_owned(),
                mapped_columns: vec!["id".into(), "name".into()],
                mapped_types: vec!["signed".into(), "text".into()],
                expected_fingerprint: import_preview.fingerprint,
                now_ms: 3_000,
            })
            .unwrap();
        std::fs::remove_file(import_path).unwrap();
        let import_operation = bridge
            .start_csv_import_apply(import_review.token_id, 3_001, session.clone())
            .unwrap();
        let import_outcome = wait_csv_import(&bridge, import_operation.clone());
        assert_eq!(import_outcome.phase, "completed", "{import_outcome:?}");
        assert_eq!(import_outcome.applied_rows, 1_200);
        assert!(bridge.dismiss_csv_import(import_operation).unwrap());
        let count_page = execute(
            &bridge,
            session.clone(),
            "SELECT count(*)::bigint FROM bridge_stream_import",
        );
        let count_page = ResultPage::decode_v1(
            &count_page,
            PageLimits::new(500, 64, 4 * 1024 * 1024, 64 * 1024),
        )
        .unwrap();
        assert_eq!(
            count_page.cell(0, 0).unwrap().bytes(),
            1_200_i64.to_be_bytes()
        );
        let export_path = std::env::temp_dir().join(format!(
            "tablerock-pg-full-export-{}.csv",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&export_path);
        let export_operation = bridge
            .start_stream_export(BridgeStreamExportRequest {
                session_id: session.clone(),
                statement: "SELECT id, name FROM bridge_stream_import ORDER BY id".into(),
                format: "csv".into(),
                path: export_path.to_string_lossy().into_owned(),
            })
            .unwrap();
        let export_outcome = wait_stream_export(&bridge, export_operation.clone());
        assert_eq!(export_outcome.phase, "completed", "{export_outcome:?}");
        assert_eq!(export_outcome.completed_rows, 1_200);
        let export_body = std::fs::read_to_string(&export_path).unwrap();
        assert!(export_body.starts_with("id,name\n"));
        assert!(export_body.contains("1199,row-1199\n"));
        assert!(bridge.dismiss_stream_export(export_operation).unwrap());
        std::fs::remove_file(export_path).unwrap();
        let drop_review = bridge
            .stage_ddl_change(BridgeDdlChangeRequest {
                session_id: session.clone(),
                catalog_node_id: relation.id_bytes.clone(),
                kind: "drop_column".into(),
                object_name: "reviewed_name".into(),
                definition: String::new(),
                now_ms: 3_000,
            })
            .unwrap();
        assert!(drop_review.destructive);
        assert!(bridge.revoke_ddl_change(drop_review.token_id).unwrap());
        let confirmation_review = bridge
            .stage_ddl_change(BridgeDdlChangeRequest {
                session_id: session.clone(),
                catalog_node_id: relation.id_bytes.clone(),
                kind: "drop_column".into(),
                object_name: "reviewed_name".into(),
                definition: String::new(),
                now_ms: 4_000,
            })
            .unwrap();
        let confirmation = bridge
            .apply_ddl_change(
                confirmation_review.token_id.clone(),
                session.clone(),
                4_001,
                false,
            )
            .unwrap_err();
        assert!(confirmation.to_string().contains("confirmation"));
        assert!(
            bridge
                .revoke_ddl_change(confirmation_review.token_id)
                .unwrap()
        );
        let expired_review = bridge
            .stage_ddl_change(BridgeDdlChangeRequest {
                session_id: session.clone(),
                catalog_node_id: relation.id_bytes.clone(),
                kind: "add_column".into(),
                object_name: "expired_column".into(),
                definition: "text".into(),
                now_ms: 5_000,
            })
            .unwrap();
        let expired = bridge
            .apply_ddl_change(expired_review.token_id, session.clone(), 65_000, true)
            .unwrap_err();
        assert!(expired.to_string().contains("expired"));
        assert!(
            !bridge
                .relation_structure(session.clone(), relation.id_bytes.clone())
                .unwrap()
                .columns
                .iter()
                .any(|column| column.name == "expired_column")
        );
        let other_session =
            open_when_ready(&bridge, "postgresql", &host, port, "postgres", "postgres");
        let scoped_review = bridge
            .stage_ddl_change(BridgeDdlChangeRequest {
                session_id: session.clone(),
                catalog_node_id: relation.id_bytes,
                kind: "drop_column".into(),
                object_name: "reviewed_name".into(),
                definition: String::new(),
                now_ms: 6_000,
            })
            .unwrap();
        let cross_session = bridge
            .apply_ddl_change(
                scoped_review.token_id.clone(),
                other_session.clone(),
                6_001,
                true,
            )
            .unwrap_err();
        assert!(cross_session.to_string().contains("another session"));
        assert!(!bridge.revoke_ddl_change(scoped_review.token_id).unwrap());
        bridge.disconnect(other_session).unwrap();
        let create_ops = bridge
            .submit(SubmitSpec {
                intent: "execute".into(),
                session_id: session.clone(),
                statement: Some("CREATE TABLE IF NOT EXISTS bridge_table_ops (id integer)".into()),
                result_id: None,
                start_row: None,
                row_count: Some(16),
                expected_revision: 0,
            })
            .unwrap();
        bridge.pump(create_ops).unwrap();
        let ops_relation = bridge
            .refresh_catalog(session.clone(), Some(schema.id_bytes.clone()))
            .unwrap()
            .into_iter()
            .find(|node| node.name == "bridge_table_ops")
            .unwrap();
        let analyze = bridge
            .stage_table_operation(BridgeTableOperationRequest {
                session_id: session.clone(),
                catalog_node_id: ops_relation.id_bytes.clone(),
                kind: "analyze".into(),
                new_name: String::new(),
                now_ms: 7_000,
            })
            .unwrap();
        assert_eq!(analyze.preview, "ANALYZE \"public\".\"bridge_table_ops\";");
        assert!(!analyze.destructive);
        assert!(
            bridge
                .apply_ddl_change(analyze.token_id.clone(), session.clone(), 7_001, true)
                .unwrap_err()
                .to_string()
                .contains("specific apply path")
        );
        assert!(
            bridge
                .apply_table_operation(
                    analyze.token_id.clone(),
                    session.clone(),
                    7_002,
                    "wrong".into(),
                )
                .unwrap_err()
                .to_string()
                .contains("exactly match")
        );
        bridge
            .apply_table_operation(
                analyze.token_id,
                session.clone(),
                7_003,
                "bridge_table_ops".into(),
            )
            .unwrap();
        let rename = bridge
            .stage_table_operation(BridgeTableOperationRequest {
                session_id: session.clone(),
                catalog_node_id: ops_relation.id_bytes,
                kind: "rename".into(),
                new_name: "bridge_table_ops_renamed".into(),
                now_ms: 8_000,
            })
            .unwrap();
        bridge
            .apply_table_operation(
                rename.token_id,
                session.clone(),
                8_001,
                "bridge_table_ops".into(),
            )
            .unwrap();
        let renamed = bridge
            .refresh_catalog(session.clone(), Some(schema.id_bytes.clone()))
            .unwrap()
            .into_iter()
            .find(|node| node.name == "bridge_table_ops_renamed")
            .unwrap();
        let drop_review = bridge
            .stage_table_operation(BridgeTableOperationRequest {
                session_id: session.clone(),
                catalog_node_id: renamed.id_bytes,
                kind: "drop".into(),
                new_name: String::new(),
                now_ms: 9_000,
            })
            .unwrap();
        assert!(drop_review.destructive);
        bridge
            .apply_table_operation(
                drop_review.token_id,
                session.clone(),
                9_001,
                "bridge_table_ops_renamed".into(),
            )
            .unwrap();
        assert!(
            bridge
                .refresh_catalog(session.clone(), Some(schema.id_bytes))
                .unwrap()
                .iter()
                .all(|node| node.name != "bridge_table_ops_renamed")
        );
        let signal = bridge
            .signal_postgres_backend(session.clone(), "cancel".into(), i32::MAX)
            .unwrap();
        assert!(
            !signal.acknowledged,
            "unknown PID must not report authority success"
        );
        let probe = bridge
            .probe_postgres_tool("dump".into(), pg_dump_explicit_path())
            .unwrap();
        if probe.available {
            let archive = std::env::temp_dir().join(format!(
                "tablerock-bridge-pg-dump-{}.dump",
                std::process::id()
            ));
            let _ = std::fs::remove_file(&archive);
            let tool_operation = bridge
                .start_postgres_tool(BridgePostgresToolRequest {
                    session_id: session.clone(),
                    kind: "dump".into(),
                    tool_path: probe.path.unwrap(),
                    file_path: archive.to_string_lossy().into_owned(),
                    content: "all".into(),
                    clean: false,
                    no_owner: false,
                })
                .unwrap();
            let status = loop {
                let status = bridge.postgres_tool_status(tool_operation.clone()).unwrap();
                if status.phase != "running" && status.phase != "cancel_requested" {
                    break status;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            };
            assert_eq!(status.phase, "succeeded", "{}", status.summary);
            assert!(archive.metadata().unwrap().len() > 0);
            std::fs::remove_file(archive).unwrap();
        } else {
            eprintln!("skip bridge pg_dump lifecycle: client tool unavailable");
        }
        let (page, _) = probe_and_fetch(&bridge, session, 0);
        assert!(!page.is_empty());
        bridge.shutdown(false, 5_000).unwrap();
    })
    .await
    .unwrap();
}

#[ignore = "real-server test: runs in CI real-servers job with --include-ignored"]
#[tokio::test]
async fn bridge_clickhouse_open_probe_fetch() {
    let container = GenericImage::new(
        "clickhouse",
        "26.3.17.4-jammy@sha256:158dcce6f6fdc59309650aad6b79484abf4eed07d4e0bdba31d732e64b5a25fb",
    )
    .with_exposed_port(8123.tcp())
    .with_env_var("CLICKHOUSE_SKIP_USER_SETUP", "1")
    .start()
    .await
    .unwrap();
    let port = container.get_host_port_ipv4(8123.tcp()).await.unwrap();
    let host = container.get_host().await.unwrap().to_string();

    for _ in 0..30 {
        if std::net::TcpStream::connect((host.as_str(), port)).is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    let (engine, page) = tokio::task::spawn_blocking(move || {
        let bridge = TableRockBridge::new_for_test();
        let session = open_when_ready(&bridge, "clickhouse", &host, port, "default", "default");
        let create = bridge
            .submit(SubmitSpec {
                intent: "execute".into(),
                session_id: session.clone(),
                statement: Some(
                    "CREATE TABLE IF NOT EXISTS bridge_optimize (id UInt64) ENGINE = MergeTree ORDER BY id"
                        .into(),
                ),
                result_id: None,
                start_row: None,
                row_count: Some(16),
                expected_revision: 0,
            })
            .unwrap();
        bridge.pump(create).unwrap();
        execute(
            &bridge,
            session.clone(),
            "CREATE TABLE IF NOT EXISTS bridge_stream_import (id Int64, name String) \
             ENGINE = MergeTree ORDER BY id",
        );
        let database = bridge
            .refresh_catalog(session.clone(), None)
            .unwrap()
            .into_iter()
            .find(|node| node.name == "default")
            .unwrap();
        let tables = bridge
            .refresh_catalog(session.clone(), Some(database.id_bytes))
            .unwrap();
        let table = tables
            .iter()
            .find(|node| node.name == "bridge_optimize")
            .unwrap()
            .clone();
        let import_table = tables
            .into_iter()
            .find(|node| node.name == "bridge_stream_import")
            .unwrap();
        let optimize = bridge
            .stage_table_operation(BridgeTableOperationRequest {
                session_id: session.clone(),
                catalog_node_id: table.id_bytes,
                kind: "optimize".into(),
                new_name: String::new(),
                now_ms: 1_000,
            })
            .unwrap();
        assert_eq!(
            optimize.preview,
            "OPTIMIZE TABLE \"default\".\"bridge_optimize\";"
        );
        bridge
            .apply_table_operation(
                optimize.token_id,
                session.clone(),
                1_001,
                "bridge_optimize".into(),
            )
            .unwrap();
        let import_path = std::env::temp_dir().join(format!(
            "tablerock-clickhouse-stream-import-{}.csv",
            std::process::id()
        ));
        let mut import_csv = String::from("id,name\n");
        for row in 0..501_u64 {
            import_csv.push_str(&format!("{row},row-{row}\n"));
        }
        std::fs::write(&import_path, import_csv).unwrap();
        let import_preview = bridge
            .preview_csv_import(import_path.to_string_lossy().into_owned())
            .unwrap();
        let import_review = bridge
            .stage_csv_import(BridgeCsvImportRequest {
                session_id: session.clone(),
                catalog_node_id: import_table.id_bytes,
                path: import_path.to_string_lossy().into_owned(),
                mapped_columns: vec!["id".into(), "name".into()],
                mapped_types: vec!["signed".into(), "text".into()],
                expected_fingerprint: import_preview.fingerprint,
                now_ms: 2_000,
            })
            .unwrap();
        std::fs::remove_file(import_path).unwrap();
        let import_operation = bridge
            .start_csv_import_apply(import_review.token_id, 2_001, session.clone())
            .unwrap();
        let import_outcome = wait_csv_import(&bridge, import_operation.clone());
        assert_eq!(import_outcome.phase, "completed", "{import_outcome:?}");
        assert_eq!(import_outcome.applied_rows, 501);
        assert!(bridge.dismiss_csv_import(import_operation).unwrap());
        let count_page = execute(
            &bridge,
            session.clone(),
            "SELECT count() FROM bridge_stream_import",
        );
        let count_page = ResultPage::decode_v1(
            &count_page,
            PageLimits::new(500, 64, 4 * 1024 * 1024, 64 * 1024),
        )
        .unwrap();
        assert_eq!(count_page.cell(0, 0).unwrap().bytes(), 501_u64.to_be_bytes());
        let export_path = std::env::temp_dir().join(format!(
            "tablerock-clickhouse-full-export-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&export_path);
        let export_operation = bridge
            .start_stream_export(BridgeStreamExportRequest {
                session_id: session.clone(),
                statement: "SELECT id, name FROM bridge_stream_import ORDER BY id".into(),
                format: "json".into(),
                path: export_path.to_string_lossy().into_owned(),
            })
            .unwrap();
        let export_outcome = wait_stream_export(&bridge, export_operation.clone());
        assert_eq!(export_outcome.phase, "completed", "{export_outcome:?}");
        assert_eq!(export_outcome.completed_rows, 501);
        let export_body = std::fs::read_to_string(&export_path).unwrap();
        assert!(export_body.contains("\"id\":500"));
        assert!(bridge.dismiss_stream_export(export_operation).unwrap());
        std::fs::remove_file(export_path).unwrap();
        let (page, _) = probe_and_fetch(&bridge, session, 0);
        let engine = ResultPage::decode_v1(
            &page,
            PageLimits::new(500, 64, 4 * 1024 * 1024, 64 * 1024),
        )
        .unwrap()
        .envelope()
        .engine();
        bridge.shutdown(false, 5_000).unwrap();
        (engine, page)
    })
    .await
    .unwrap();
    assert_eq!(engine, Engine::ClickHouse);
    assert!(!page.is_empty());
}

#[ignore = "real-server test: runs in CI real-servers job with --include-ignored"]
#[tokio::test]
async fn bridge_redis_open_probe_fetch() {
    let container = GenericImage::new(
        "redis",
        "8.8.0-alpine@sha256:9d317178eceac8454a2284a9e6df2466b93c745529947f0cd42a0fa9609d7005",
    )
    .with_exposed_port(6379.tcp())
    .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
    .start()
    .await
    .unwrap();
    let port = container.get_host_port_ipv4(6379.tcp()).await.unwrap();
    let host = container.get_host().await.unwrap().to_string();

    {
        use redis::AsyncCommands;
        let client = redis::Client::open(format!("redis://{host}:{port}")).unwrap();
        let mut last = None;
        for _ in 0..50 {
            match client.get_multiplexed_async_connection().await {
                Ok(mut conn) => {
                    let _: () = conn.set("bridge:probe", "1").await.unwrap();
                    last = None;
                    break;
                }
                Err(error) => {
                    last = Some(error);
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
        }
        if let Some(error) = last {
            panic!("redis seed failed: {error}");
        }
    }

    let publish_host = host.clone();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let publisher = tokio::spawn(async move {
        ready_rx.await.unwrap();
        let client = redis::Client::open(format!("redis://{publish_host}:{port}")).unwrap();
        let mut connection = client.get_multiplexed_async_connection().await.unwrap();
        let delivered: u64 = redis::cmd("PUBLISH")
            .arg("bridge:events")
            .arg("facade-message")
            .query_async(&mut connection)
            .await
            .unwrap();
        assert_eq!(delivered, 1);
    });
    let (engine, page) = tokio::task::spawn_blocking(move || {
        let bridge = TableRockBridge::new_for_test();
        let session = open_when_ready(&bridge, "redis", &host, port, "0", "");
        let subscription = bridge
            .start_redis_subscription(session.clone(), "bridge:events".into(), false)
            .unwrap();
        loop {
            let status = bridge
                .redis_subscription_status(subscription.clone())
                .unwrap();
            if status.phase == "listening" {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        ready_tx.send(()).unwrap();
        let status = loop {
            let status = bridge
                .redis_subscription_status(subscription.clone())
                .unwrap();
            if status.total_received > 0 {
                break status;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        assert_eq!(status.messages, vec!["bridge:events · facade-message"]);
        assert_eq!(status.discontinuities, 0);
        assert!(
            bridge
                .cancel_redis_subscription(subscription.clone())
                .unwrap()
        );
        loop {
            let status = bridge
                .redis_subscription_status(subscription.clone())
                .unwrap();
            if status.phase == "cancelled" {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let (page, _) = probe_and_fetch(&bridge, session, 0);
        let engine =
            ResultPage::decode_v1(&page, PageLimits::new(500, 64, 4 * 1024 * 1024, 64 * 1024))
                .unwrap()
                .envelope()
                .engine();
        bridge.shutdown(false, 5_000).unwrap();
        (engine, page)
    })
    .await
    .unwrap();
    publisher.await.unwrap();
    assert_eq!(engine, Engine::Redis);
    assert!(!page.is_empty());
}

#[ignore = "real-server test: runs in CI real-servers job with --include-ignored"]
#[tokio::test]
async fn bridge_postgres_apply_delete_by_review_token() {
    let container = GenericImage::new("postgres", "18.4-alpine")
        .with_exposed_port(5432.tcp())
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_HOST_AUTH_METHOD", "trust")
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(5432.tcp()).await.unwrap();
    let host = container.get_host().await.unwrap().to_string();

    let outcome = tokio::task::spawn_blocking(move || {
        let bridge = TableRockBridge::new_for_test();
        let session = bridge
            .open(open_params(
                "postgresql",
                &host,
                port,
                "postgres",
                "postgres",
            ))
            .expect("open");
        for sql in [
            "create table if not exists bridge_apply_probe (id int primary key)",
            "delete from bridge_apply_probe",
            "insert into bridge_apply_probe (id) values (7)",
        ] {
            let op = bridge
                .submit(SubmitSpec {
                    intent: "execute".into(),
                    session_id: session.clone(),
                    statement: Some(sql.into()),
                    result_id: None,
                    start_row: None,
                    row_count: Some(16),
                    expected_revision: 0,
                })
                .unwrap_or_else(|e| panic!("submit {sql}: {e}"));
            bridge.pump(op).unwrap_or_else(|e| panic!("pump {sql}: {e}"));
        }
        let now = 1_000_u64;
        let token = bridge
            .insert_reviewed_probe(
                session.clone(),
                now,
                now + 30_000,
                now + 100,
                Some("bridge_apply_probe".into()),
                Some(7),
            )
            .expect("review token");
        let applied = bridge
            .apply_review_token(token.clone(), now + 200, session.clone(), 0)
            .expect("apply");
        assert!(
            applied.applied_count >= 1 || applied.transaction.contains("Committed"),
            "apply outcome: {applied:?}"
        );
        // Handle consumed — second apply must fail.
        let again = bridge
            .apply_review_token(token, now + 300, session.clone(), 0)
            .expect_err("second apply must fail");
        assert!(
            matches!(again, tablerock_ffi::BridgeError::Rejected { ref code, .. } if code == "authorize"),
            "second apply: {again:?}"
        );
        bridge.shutdown(false, 5_000).expect("shutdown");
        applied
    })
    .await
    .unwrap();
    assert!(outcome.change_count >= 1);
}

#[ignore = "real-server test: runs in CI real-servers job with --include-ignored"]
#[tokio::test]
async fn bridge_three_engines_sequential_open_probe() {
    let postgres = GenericImage::new("postgres", "18.4-alpine")
        .with_exposed_port(5432.tcp())
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_HOST_AUTH_METHOD", "trust")
        .start();
    let clickhouse = GenericImage::new(
        "clickhouse",
        "26.3.17.4-jammy@sha256:158dcce6f6fdc59309650aad6b79484abf4eed07d4e0bdba31d732e64b5a25fb",
    )
    .with_exposed_port(8123.tcp())
    .with_env_var("CLICKHOUSE_SKIP_USER_SETUP", "1")
    .start();
    let redis = GenericImage::new(
        "redis",
        "8.8.0-alpine@sha256:9d317178eceac8454a2284a9e6df2466b93c745529947f0cd42a0fa9609d7005",
    )
    .with_exposed_port(6379.tcp())
    .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
    .start();
    let (postgres, clickhouse, redis) = tokio::join!(postgres, clickhouse, redis);
    let postgres = postgres.unwrap();
    let clickhouse = clickhouse.unwrap();
    let redis = redis.unwrap();
    let pg_port = postgres.get_host_port_ipv4(5432.tcp()).await.unwrap();
    let ch_port = clickhouse.get_host_port_ipv4(8123.tcp()).await.unwrap();
    let redis_port = redis.get_host_port_ipv4(6379.tcp()).await.unwrap();
    let pg_host = postgres.get_host().await.unwrap().to_string();
    let ch_host = clickhouse.get_host().await.unwrap().to_string();
    let redis_host = redis.get_host().await.unwrap().to_string();

    {
        use redis::AsyncCommands;
        let client = redis::Client::open(format!("redis://{redis_host}:{redis_port}")).unwrap();
        let mut conn = client.get_multiplexed_async_connection().await.unwrap();
        let _: () = conn.set("bridge:three", "ok").await.unwrap();
    }

    // Warm ClickHouse HTTP before bridge probes.
    for attempt in 0..60 {
        if std::net::TcpStream::connect((ch_host.as_str(), ch_port)).is_ok() {
            break;
        }
        assert!(attempt < 59, "clickhouse port never accepted TCP");
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let engines = tokio::task::spawn_blocking(move || {
        let bridge = TableRockBridge::new_for_test();
        let mut observed = Vec::new();
        let mut cursor = 0_u64;
        for (engine, host, port, db, user) in [
            ("postgresql", pg_host, pg_port, "postgres", "postgres"),
            ("clickhouse", ch_host, ch_port, "default", "default"),
            ("redis", redis_host, redis_port, "0", ""),
        ] {
            let session = open_when_ready(&bridge, engine, &host, port, db, user);
            let (page, next) = probe_and_fetch(&bridge, session, cursor);
            cursor = next;
            let decoded =
                ResultPage::decode_v1(&page, PageLimits::new(500, 64, 4 * 1024 * 1024, 64 * 1024))
                    .unwrap();
            observed.push(decoded.envelope().engine());
        }
        bridge.shutdown(false, 5_000).expect("shutdown");
        observed
    })
    .await
    .unwrap();

    assert_eq!(
        engines,
        vec![Engine::PostgreSql, Engine::ClickHouse, Engine::Redis]
    );
}
