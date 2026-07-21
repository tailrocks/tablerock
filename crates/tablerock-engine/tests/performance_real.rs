use std::{
    process::Command,
    time::{Duration, Instant},
};

use redis::AsyncCommands;
use tablerock_core::{
    BoundedText, ByteLimit, Engine, IdParts, PageIdentity, PageLimits, ResultId, Revision,
};
use tablerock_engine::{
    ClickHouseCompression, ClickHouseConnectConfig, ClickHouseProbeQuery, ClickHouseSession,
    ClickHouseTlsMode, DriverPageRequest, DriverSession, PostgresConnectConfig, PostgresProbeQuery,
    PostgresSession, PostgresTlsMode, RedisConnectConfig, RedisConnectionSecurity, RedisProtocol,
    RedisSession, RedisTlsMode,
};
use testcontainers::{
    GenericImage, ImageExt,
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};

const ROWS: u64 = 10_000;
const FIRST_PAGE_BUDGET: Duration = Duration::from_secs(5);
const TOTAL_BUDGET: Duration = Duration::from_secs(15);
const MIN_ROWS_PER_SECOND: f64 = 500.0;
const MAX_PAGE_RESIDENT_BYTES: u64 = 2 * 1024 * 1024;
const MAX_PROCESS_RSS_BYTES: u64 = 512 * 1024 * 1024;

fn text(value: &str) -> BoundedText {
    BoundedText::copy_from_str(value, ByteLimit::new(128)).unwrap()
}

#[tokio::test]
async fn current_servers_meet_initial_streaming_budgets() {
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
    let postgres_port = postgres.get_host_port_ipv4(5432.tcp()).await.unwrap();
    let clickhouse_port = clickhouse.get_host_port_ipv4(8123.tcp()).await.unwrap();
    let redis_port = redis.get_host_port_ipv4(6379.tcp()).await.unwrap();
    let postgres_host = postgres.get_host().await.unwrap().to_string();
    let clickhouse_host = clickhouse.get_host().await.unwrap().to_string();
    let redis_host = redis.get_host().await.unwrap().to_string();
    seed_redis(&redis_host, redis_port).await;

    let postgres = PostgresSession::connect(&PostgresConnectConfig::new(
        text(&postgres_host),
        postgres_port,
        text("postgres"),
        text("postgres"),
        PostgresTlsMode::Disabled,
    ))
    .await
    .unwrap();
    let clickhouse = ready_clickhouse(&clickhouse_host, clickhouse_port).await;
    let redis = RedisSession::connect(
        &RedisConnectConfig::new(
            text(&redis_host),
            redis_port,
            0,
            RedisProtocol::Resp3,
            RedisTlsMode::Disable,
        ),
        RedisConnectionSecurity::new(),
    )
    .await
    .unwrap();

    let rss_before = process_rss_bytes().expect("ps reports process RSS");
    let postgres = measure(
        &postgres,
        DriverPageRequest::PostgreSqlProbe {
            query: PostgresProbeQuery::PerformanceSeries,
            limits: limits(),
            max_cell_bytes: 64,
        },
        identity(Engine::PostgreSql, 201),
        ROWS,
    )
    .await;
    let clickhouse = measure(
        &clickhouse,
        DriverPageRequest::ClickHouseProbe {
            query: ClickHouseProbeQuery::PerformanceSeries,
            query_id: text(&format!("tablerock-performance-{clickhouse_port}")),
            limits: limits(),
            max_cell_bytes: 64,
        },
        identity(Engine::ClickHouse, 202),
        ROWS,
    )
    .await;
    let redis = measure(
        &redis,
        DriverPageRequest::RedisKeyScan {
            limits: limits(),
            max_cell_bytes: 64,
            scan_count: 500,
            max_scan_rounds: 1_000,
            match_pattern: None,
        },
        identity(Engine::Redis, 203),
        ROWS,
    )
    .await;
    let rss_after = process_rss_bytes().expect("ps reports process RSS");

    for evidence in [postgres, clickhouse, redis] {
        assert!(matches!(
            evidence.engine,
            Engine::PostgreSql | Engine::ClickHouse | Engine::Redis
        ));
        assert!(evidence.first_page <= FIRST_PAGE_BUDGET, "{evidence:?}");
        assert!(evidence.total <= TOTAL_BUDGET, "{evidence:?}");
        assert!(evidence.rows >= ROWS, "{evidence:?}");
        assert!(
            evidence.rows_per_second >= MIN_ROWS_PER_SECOND,
            "{evidence:?}"
        );
        assert!(
            evidence.max_page_resident_bytes <= MAX_PAGE_RESIDENT_BYTES,
            "{evidence:?}"
        );
        eprintln!("{evidence:?}");
    }
    let rss = rss_before.max(rss_after);
    assert!(
        rss <= MAX_PROCESS_RSS_BYTES,
        "process RSS {rss} exceeds budget"
    );
    eprintln!("process_rss_bytes={rss}");
}

const fn limits() -> PageLimits {
    PageLimits::new(500, 8, 1024 * 1024, 64 * 1024)
}

fn identity(engine: Engine, low: u64) -> PageIdentity {
    PageIdentity::new(
        ResultId::from_parts(IdParts::new(0, low).unwrap()).unwrap(),
        Revision::INITIAL,
        engine,
    )
}

#[derive(Debug)]
struct Evidence {
    engine: Engine,
    rows: u64,
    first_page: Duration,
    total: Duration,
    rows_per_second: f64,
    max_page_resident_bytes: u64,
}

async fn measure(
    session: &dyn DriverSession,
    request: DriverPageRequest,
    identity: PageIdentity,
    expected_rows: u64,
) -> Evidence {
    let engine = request.engine();
    let started = Instant::now();
    let mut stream = session.start_page_stream(request).await.unwrap();
    let mut rows = 0_u64;
    let mut first_page = None;
    let mut max_page_resident_bytes = 0_u64;
    loop {
        let page = tokio::time::timeout(TOTAL_BUDGET, stream.next_page(identity, rows))
            .await
            .expect("page remains inside the total streaming budget")
            .unwrap();
        let Some(page) = page else { break };
        first_page.get_or_insert_with(|| started.elapsed());
        assert!(page.envelope().row_count() <= 500);
        rows += u64::from(page.envelope().row_count());
        max_page_resident_bytes = max_page_resident_bytes.max(page.resident_buffer_bytes());
    }
    let total = started.elapsed();
    assert!(rows >= expected_rows, "{engine:?} returned {rows} rows");
    Evidence {
        engine,
        rows,
        first_page: first_page.expect("stream returns at least one page"),
        total,
        rows_per_second: rows as f64 / total.as_secs_f64(),
        max_page_resident_bytes,
    }
}

async fn ready_clickhouse(host: &str, port: u16) -> ClickHouseSession {
    let session = ClickHouseSession::connect(&ClickHouseConnectConfig::new(
        text(host),
        port,
        text("default"),
        text("default"),
        ClickHouseTlsMode::Disable,
        ClickHouseCompression::Lz4,
    ));
    for attempt in 0..300 {
        let result = session
            .start_page_stream(DriverPageRequest::ClickHouseProbe {
                query: ClickHouseProbeQuery::TypedValues,
                query_id: text(&format!("tablerock-performance-ready-{port}-{attempt}")),
                limits: PageLimits::new(1, 8, 256, 256),
                max_cell_bytes: 32,
            })
            .await;
        if let Ok(stream) = result {
            drop(stream);
            return session;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("ClickHouse fixture did not become ready");
}

async fn seed_redis(host: &str, port: u16) {
    let client = redis::Client::open(format!("redis://{host}:{port}/0")).unwrap();
    let mut connection = client.get_multiplexed_async_connection().await.unwrap();
    for index in 0..ROWS {
        let key = format!("performance-{index:05}");
        let _: () = connection.set(key, "value").await.unwrap();
    }
}

fn process_rss_bytes() -> Option<u64> {
    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let kib = std::str::from_utf8(&output.stdout)
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()?;
    kib.checked_mul(1024)
}
