//! Real-server bridge path: open → submit probe → pump → fetch_page → shutdown
//! through the synchronous UniFFI facade against Docker engines.
//!
//! The facade owns a multi-thread Tokio runtime and uses `block_on`. Tests start
//! containers on the async test runtime, then call the bridge from
//! `spawn_blocking` so runtimes never nest.

use tablerock_core::{Engine, PageLimits, ResultPage};
use tablerock_ffi::{OpenParams, SubmitSpec, TableRockBridge};
use testcontainers::{
    GenericImage, ImageExt,
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};

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

fn run_bridge_probe(
    engine: &str,
    host: &str,
    port: u16,
    database: &str,
    user: &str,
) -> (Engine, Vec<u8>) {
    let bridge = TableRockBridge::new_for_test();
    let mut last_err = None;
    for attempt in 0..40 {
        match bridge.open(open_params(engine, host, port, database, user)) {
            Ok(session) => match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                probe_and_fetch(&bridge, session, 0)
            })) {
                Ok((page, _)) => {
                    let decoded = ResultPage::decode_v1(
                        &page,
                        PageLimits::new(500, 64, 4 * 1024 * 1024, 64 * 1024),
                    )
                    .unwrap();
                    let observed = decoded.envelope().engine();
                    bridge.shutdown(false, 5_000).expect("shutdown");
                    return (observed, page);
                }
                Err(payload) => {
                    last_err = Some(format!("probe panic attempt {attempt}: {payload:?}"));
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
            },
            Err(error) => {
                last_err = Some(format!("open attempt {attempt}: {error}"));
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        }
    }
    panic!("bridge probe failed for {engine}: {last_err:?}");
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
        let session = open_when_ready(&bridge, "postgresql", &host, port, "postgres", "postgres");
        let activity = bridge.postgres_activity(session.clone()).unwrap();
        assert!(!activity.is_empty());
        assert!(activity.iter().all(|row| row.pid > 0));
        let signal = bridge
            .signal_postgres_backend(session.clone(), "cancel".into(), i32::MAX)
            .unwrap();
        assert!(
            !signal.acknowledged,
            "unknown PID must not report authority success"
        );
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
        run_bridge_probe("clickhouse", &host, port, "default", "default")
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

    let (engine, page) =
        tokio::task::spawn_blocking(move || run_bridge_probe("redis", &host, port, "0", ""))
            .await
            .unwrap();
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
