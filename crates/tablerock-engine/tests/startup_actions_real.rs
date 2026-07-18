//! Real-server proof: startup actions on PG/CH/Redis.

use tablerock_core::{
    BoundedText, ByteLimit, StartupAction, StartupActionOutcome, StartupActionSet,
    StartupSafetyClass,
};
use tablerock_engine::{
    ClickHouseCompression, ClickHouseConnectConfig, ClickHouseSession, ClickHouseTlsMode,
    PostgresConnectConfig, PostgresSession, PostgresTlsMode, RedisConnectConfig,
    RedisConnectionSecurity, RedisProtocol, RedisSession, RedisTlsMode,
    run_clickhouse_startup_actions, run_postgres_startup_actions, run_redis_startup_actions,
};
use testcontainers::{
    GenericImage, ImageExt,
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};

fn text(s: &str) -> BoundedText {
    BoundedText::copy_from_str(s, ByteLimit::new(253)).unwrap()
}

#[tokio::test]
async fn postgres_startup_actions_auto_run_and_skip_writes() {
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
    let session = PostgresSession::connect(&PostgresConnectConfig::new(
        text("127.0.0.1"),
        port,
        text("postgres"),
        text("postgres"),
        PostgresTlsMode::Disabled,
    ))
    .await
    .unwrap();

    let set = StartupActionSet::new(vec![
        StartupAction::from_str("SELECT 1", StartupSafetyClass::ReadOnly, 5_000, true).unwrap(),
        StartupAction::from_str(
            "CREATE TABLE IF NOT EXISTS startup_probe (id int)",
            StartupSafetyClass::Write,
            5_000,
            true,
        )
        .unwrap(),
        StartupAction::from_str("SELECT 1/0", StartupSafetyClass::ReadOnly, 5_000, true).unwrap(),
    ])
    .unwrap();

    let report = run_postgres_startup_actions(&session, &set, false).await;
    let outcomes: Vec<_> = report.outcomes().iter().map(|(_, o)| *o).collect();
    assert_eq!(
        outcomes,
        vec![
            StartupActionOutcome::Succeeded,
            StartupActionOutcome::SkippedNeedsReview,
            StartupActionOutcome::Failed, // division by zero
        ]
    );
    assert!(report.has_failure());

    // Reconnect filter: Write-only-on-initial would be skipped on reconnect;
    // here all three have run_on_reconnect true, still skip Write.
    let reconnect = run_postgres_startup_actions(&session, &set, true).await;
    assert_eq!(
        reconnect.outcomes()[1].1,
        StartupActionOutcome::SkippedNeedsReview
    );

    session.shutdown().await.ok();
}

#[tokio::test]
async fn clickhouse_startup_actions_auto_run() {
    let image =
        "26.3.17.4-jammy@sha256:158dcce6f6fdc59309650aad6b79484abf4eed07d4e0bdba31d732e64b5a25fb";
    let container = GenericImage::new("clickhouse", image)
        .with_exposed_port(8123.tcp())
        .with_env_var("CLICKHOUSE_SKIP_USER_SETUP", "1")
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(8123.tcp()).await.unwrap();
    let session = ClickHouseSession::connect(&ClickHouseConnectConfig::new(
        text("127.0.0.1"),
        port,
        text("default"),
        text("default"),
        ClickHouseTlsMode::Disable,
        ClickHouseCompression::None,
    ));
    // Wait until CH accepts queries.
    for _ in 0..50 {
        if session.health_check().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let set = StartupActionSet::new(vec![
        StartupAction::from_str("SELECT 1", StartupSafetyClass::ReadOnly, 10_000, true).unwrap(),
        StartupAction::from_str(
            "CREATE TABLE IF NOT EXISTS startup_skip (x UInt8) ENGINE = Memory",
            StartupSafetyClass::Write,
            10_000,
            true,
        )
        .unwrap(),
    ])
    .unwrap();
    let report = run_clickhouse_startup_actions(&session, &set, false).await;
    assert_eq!(
        report
            .outcomes()
            .iter()
            .map(|(_, o)| *o)
            .collect::<Vec<_>>(),
        vec![
            StartupActionOutcome::Succeeded,
            StartupActionOutcome::SkippedNeedsReview,
        ]
    );
}

#[tokio::test]
async fn redis_startup_actions_auto_run() {
    let container = GenericImage::new("redis", "8.8.0")
        .with_exposed_port(6379.tcp())
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(6379.tcp()).await.unwrap();
    let session = RedisSession::connect(
        &RedisConnectConfig::new(
            text("127.0.0.1"),
            port,
            0,
            RedisProtocol::Resp3,
            RedisTlsMode::Disable,
        ),
        RedisConnectionSecurity::new(),
    )
    .await
    .unwrap();
    let set = StartupActionSet::new(vec![
        StartupAction::from_str("PING", StartupSafetyClass::ReadOnly, 5_000, true).unwrap(),
        StartupAction::from_str("FLUSHDB", StartupSafetyClass::Dangerous, 5_000, true).unwrap(),
    ])
    .unwrap();
    let report = run_redis_startup_actions(&session, &set, false).await;
    assert_eq!(
        report
            .outcomes()
            .iter()
            .map(|(_, o)| *o)
            .collect::<Vec<_>>(),
        vec![
            StartupActionOutcome::Succeeded,
            StartupActionOutcome::SkippedNeedsReview,
        ]
    );
}
