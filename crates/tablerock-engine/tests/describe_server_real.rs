//! Real-server Test Connection describe facts (plan 006).

use std::time::Duration;

use tablerock_core::{BoundedText, ByteLimit, Engine};
use tablerock_engine::{
    ClickHouseCompression, ClickHouseConnectConfig, ClickHouseSession, ClickHouseTlsMode,
    PostgresConnectConfig, PostgresSession, PostgresTlsMode, RedisConnectConfig,
    RedisConnectionSecurity, RedisProtocol, RedisSession, RedisTlsMode,
};
use testcontainers::{
    GenericImage, ImageExt,
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};

fn text(value: &str) -> BoundedText {
    BoundedText::copy_from_str(value, ByteLimit::new(128)).unwrap()
}

#[tokio::test]
async fn postgres_describe_server_returns_version_identity() {
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
    let config = PostgresConnectConfig::new(
        text("127.0.0.1"),
        port,
        text("postgres"),
        text("postgres"),
        PostgresTlsMode::Disabled,
    );
    let session = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if let Ok(session) = PostgresSession::connect(&config).await {
                break session;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap();
    let described = session.describe_server().await.unwrap();
    assert_eq!(described.engine(), Engine::PostgreSql);
    assert!(
        described
            .identity()
            .to_ascii_lowercase()
            .contains("postgres"),
        "{}",
        described.identity()
    );
    session.shutdown().await.unwrap();
}

#[tokio::test]
async fn clickhouse_describe_server_returns_version_identity() {
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
    let described = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            match session.describe_server().await {
                Ok(described) => break described,
                Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(described.engine(), Engine::ClickHouse);
    assert!(
        !described.identity().is_empty(),
        "identity should not be empty"
    );
}

#[tokio::test]
async fn redis_describe_server_returns_version_identity() {
    let container = GenericImage::new("redis", "8.8.0")
        .with_exposed_port(6379.tcp())
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(6379.tcp()).await.unwrap();
    let session = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if let Ok(session) = RedisSession::connect(
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
            {
                break session;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap();
    let described = session.describe_server().await.unwrap();
    assert_eq!(described.engine(), Engine::Redis);
    assert!(
        described.identity().to_ascii_lowercase().contains("redis"),
        "{}",
        described.identity()
    );
}
