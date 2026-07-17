use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};

use redis::AsyncCommands;
use tablerock_core::{
    BoundedText, ByteLimit, Engine, OperationOutcome, OperationPhase, PageLimits,
};
use tablerock_engine::{
    ClickHouseCompression, ClickHouseConnectConfig, ClickHouseProbeQuery, ClickHouseSession,
    ClickHouseTlsMode, DriverPageRequest, DriverSession, EngineService, EngineServiceUpdate,
    PostgresConnectConfig, PostgresProbeQuery, PostgresSession, PostgresTlsMode,
    RedisConnectConfig, RedisConnectionSecurity, RedisProtocol, RedisSession, RedisTlsMode,
};
use testcontainers::{
    GenericImage, ImageExt,
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};

mod support;

const UPDATE_DEADLINE: Duration = Duration::from_secs(30);

fn text(value: &str) -> BoundedText {
    BoundedText::copy_from_str(value, ByteLimit::new(128)).unwrap()
}

#[tokio::test]
async fn overlaps_postgres_clickhouse_and_redis_through_one_service() {
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
    seed_redis(redis_port).await;

    let postgres = PostgresSession::connect(&PostgresConnectConfig::new(
        text("127.0.0.1"),
        postgres_port,
        text("postgres"),
        text("postgres"),
        PostgresTlsMode::Disabled,
    ))
    .await
    .unwrap();
    let clickhouse = ready_clickhouse(clickhouse_port).await;
    let redis = RedisSession::connect(
        &RedisConnectConfig::new(
            text("127.0.0.1"),
            redis_port,
            0,
            RedisProtocol::Resp3,
            RedisTlsMode::Disable,
        ),
        RedisConnectionSecurity::new(),
    )
    .await
    .unwrap();

    let postgres_operation = support::operation(101);
    let clickhouse_operation = support::operation(102);
    let redis_operation = support::operation(103);
    let mut service = support::service(3, 2);
    let submitted_at = Instant::now();
    service
        .submit(
            postgres_operation,
            support::command(111),
            Box::new(postgres),
            DriverPageRequest::PostgreSqlProbe {
                query: PostgresProbeQuery::BoundedSeries,
                limits: PageLimits::new(2, 8, 256, 256),
                max_cell_bytes: 32,
            },
            support::identity(Engine::PostgreSql, 101),
        )
        .await
        .unwrap();
    service
        .submit(
            clickhouse_operation,
            support::command(112),
            Box::new(clickhouse),
            DriverPageRequest::ClickHouseProbe {
                query: ClickHouseProbeQuery::TypedValues,
                query_id: text(&format!("tablerock-overlap-{clickhouse_port}")),
                limits: PageLimits::new(2, 8, 256, 256),
                max_cell_bytes: 32,
            },
            support::identity(Engine::ClickHouse, 102),
        )
        .await
        .unwrap();
    service
        .submit(
            redis_operation,
            support::command(113),
            Box::new(redis),
            DriverPageRequest::RedisKeyScan {
                limits: PageLimits::new(2, 1, 256, 64),
                max_cell_bytes: 128,
                scan_count: 2,
                max_scan_rounds: 128,
            },
            support::identity(Engine::Redis, 103),
        )
        .await
        .unwrap();

    assert_eq!(service.core().active_operations(), 3);
    for operation in [postgres_operation, clickhouse_operation, redis_operation] {
        assert_eq!(
            service.core().operation_phase(operation),
            Some(OperationPhase::Queued)
        );
    }

    let postgres = drain(
        &mut service,
        postgres_operation,
        Engine::PostgreSql,
        submitted_at,
    )
    .await;
    let clickhouse = drain(
        &mut service,
        clickhouse_operation,
        Engine::ClickHouse,
        submitted_at,
    )
    .await;
    let redis = drain(&mut service, redis_operation, Engine::Redis, submitted_at).await;

    for evidence in [&postgres, &clickhouse, &redis] {
        assert!(evidence.first_page < UPDATE_DEADLINE);
        assert!(evidence.completed < UPDATE_DEADLINE);
        eprintln!(
            "{:?}: first page {:?}, completed {:?}",
            evidence.engine, evidence.first_page, evidence.completed
        );
    }

    assert_eq!(
        postgres.rows,
        BTreeSet::from([b"1".to_vec(), b"2".to_vec(), b"3".to_vec()])
    );
    assert_eq!(
        clickhouse.rows,
        BTreeSet::from([
            0_u64.to_be_bytes().to_vec(),
            1_u64.to_be_bytes().to_vec(),
            2_u64.to_be_bytes().to_vec()
        ])
    );
    assert_eq!(
        redis.rows,
        BTreeSet::from([
            b"overlap-a".to_vec(),
            b"overlap-b".to_vec(),
            b"overlap-c".to_vec()
        ])
    );
    assert_eq!(service.core().active_operations(), 0);
}

async fn ready_clickhouse(port: u16) -> ClickHouseSession {
    let session = ClickHouseSession::connect(&ClickHouseConnectConfig::new(
        text("127.0.0.1"),
        port,
        text("default"),
        text("default"),
        ClickHouseTlsMode::Disable,
        ClickHouseCompression::Lz4,
    ));
    let mut last_error = None;
    for attempt in 0..300 {
        match session
            .start_page_stream(DriverPageRequest::ClickHouseProbe {
                query: ClickHouseProbeQuery::TypedValues,
                query_id: text(&format!("tablerock-overlap-ready-{port}-{attempt}")),
                limits: PageLimits::new(1, 8, 256, 256),
                max_cell_bytes: 32,
            })
            .await
        {
            Ok(stream) => {
                drop(stream);
                return session;
            }
            Err(error) => {
                last_error = Some(error);
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
    panic!("ClickHouse fixture accepts HTTP queries: {last_error:?}");
}

async fn seed_redis(port: u16) {
    let client = redis::Client::open(format!("redis://127.0.0.1:{port}/0")).unwrap();
    let mut connection = client.get_multiplexed_async_connection().await.unwrap();
    for key in ["overlap-a", "overlap-b", "overlap-c"] {
        let _: () = connection.set(key, "value").await.unwrap();
    }
}

async fn drain(
    service: &mut EngineService,
    operation: tablerock_core::OperationId,
    engine: Engine,
    submitted_at: Instant,
) -> DrainEvidence {
    let mut rows = BTreeSet::new();
    let mut first_page = None;
    loop {
        let update = tokio::time::timeout(UPDATE_DEADLINE, service.next_update(operation))
            .await
            .expect("operation update stays within the overlap deadline")
            .unwrap()
            .unwrap();
        match update {
            EngineServiceUpdate::Started => {}
            EngineServiceUpdate::Page(page) => {
                first_page.get_or_insert_with(|| submitted_at.elapsed());
                assert_eq!(page.envelope().engine(), engine);
                for row in 0..page.envelope().row_count() {
                    rows.insert(page.cell(row, 0).unwrap().bytes().to_vec());
                }
            }
            EngineServiceUpdate::Terminal(OperationOutcome::Completed) => {
                return DrainEvidence {
                    engine,
                    rows,
                    first_page: first_page.expect("completed operation delivered a page"),
                    completed: submitted_at.elapsed(),
                };
            }
            other => panic!("unexpected {engine:?} overlap event: {other:?}"),
        }
    }
}

struct DrainEvidence {
    engine: Engine,
    rows: BTreeSet<Vec<u8>>,
    first_page: Duration,
    completed: Duration,
}
