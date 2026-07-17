use std::{collections::BTreeSet, sync::Arc, time::Duration};

use rcgen::ExtendedKeyUsagePurpose;
use redis::{
    AsyncCommands, Client, ClientTlsConfig, ConnectionAddr, IntoConnectionInfo, ProtocolVersion,
    RedisConnectionInfo, TlsCertificates,
};
use tablerock_core::{
    AuthorizedMutationPlan, BoundedBytes, BoundedText, ByteLimit, CancelDispatch, ContextId,
    Engine, IdParts, MutationChange, MutationId, MutationPlan, MutationPlanLimits,
    MutationReviewRegistry, MutationTarget, OperationScope, PageDelivery, PageIdentity, PageLimits,
    PageWarning, ProfileId, RedisExpiration, ResultPage, ReviewTokenId, Revision, SessionId,
    Truncation, ValueKind,
};
use tablerock_engine::{
    AdapterFailureClass, DriverPageRequest, DriverSession, EngineServiceUpdate,
    RedisClientIdentity, RedisCollectionScanKind, RedisCollectionScanOptions, RedisConnectConfig,
    RedisConnectionSecurity, RedisCredentials, RedisProtocol, RedisRuntimePolicy, RedisSession,
    RedisSubscriptionKind, RedisSubscriptionOptions, RedisTlsMaterial, RedisTlsMode,
    RedisTtlApplication,
};

struct RedisTlsFixture {
    ca_pem: String,
    wrong_ca_pem: String,
    server_certificate_pem: String,
    server_private_key_pem: String,
    client_certificate_pem: String,
    client_private_key_pem: String,
}

impl RedisTlsFixture {
    fn generate() -> Self {
        let ca = certificate_authority("TableRock Redis test CA");
        let wrong_ca = certificate_authority("Untrusted TableRock Redis test CA");
        let (server_certificate_pem, server_private_key_pem) =
            leaf_certificate("127.0.0.1", ExtendedKeyUsagePurpose::ServerAuth, &ca);
        let (client_certificate_pem, client_private_key_pem) =
            leaf_certificate("tablerock-client", ExtendedKeyUsagePurpose::ClientAuth, &ca);
        Self {
            ca_pem: ca.pem(),
            wrong_ca_pem: wrong_ca.pem(),
            server_certificate_pem,
            server_private_key_pem,
            client_certificate_pem,
            client_private_key_pem,
        }
    }
}

const REDIS_TEST_USERNAME: &str = "tablerock-test-user";
const REDIS_TEST_PASSWORD: &str = "synthetic-test-password";
const REDIS_ADMIN_USERNAME: &str = "tablerock-test-admin";
const REDIS_ADMIN_PASSWORD: &str = "synthetic-admin-password";
const REDIS_RESTRICTED_USERNAME: &str = "tablerock-restricted-user";
const REDIS_RESTRICTED_PASSWORD: &str = "synthetic-restricted-password";
const REDIS_PATTERN_USERNAME: &str = "tablerock-pattern-user";
const REDIS_PATTERN_PASSWORD: &str = "synthetic-pattern-password";

fn redis_acl_file() -> Vec<u8> {
    format!(
        "user default off\nuser {REDIS_TEST_USERNAME} reset on >{REDIS_TEST_PASSWORD} ~* &* +@all\nuser {REDIS_ADMIN_USERNAME} reset on >{REDIS_ADMIN_PASSWORD} ~* &* +@all\nuser {REDIS_RESTRICTED_USERNAME} reset on >{REDIS_RESTRICTED_PASSWORD} ~* &allowed:* +@all\nuser {REDIS_PATTERN_USERNAME} reset on >{REDIS_PATTERN_PASSWORD} ~* &* +@all\n"
    )
        .into_bytes()
}

fn redis_rotated_test_acl_file() -> Vec<u8> {
    format!(
        "user default off\nuser {REDIS_TEST_USERNAME} reset on >synthetic-replacement-password ~* &* +@all\nuser {REDIS_ADMIN_USERNAME} reset on >{REDIS_ADMIN_PASSWORD} ~* &* +@all\nuser {REDIS_RESTRICTED_USERNAME} reset on >{REDIS_RESTRICTED_PASSWORD} ~* &allowed:* +@all\nuser {REDIS_PATTERN_USERNAME} reset on >{REDIS_PATTERN_PASSWORD} ~* &* +@all\n"
    )
    .into_bytes()
}

mod support;
mod tls_support;
use testcontainers::{
    ContainerAsync, CopyDataSource, CopyTargetOptions, GenericImage, ImageExt,
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};
use tls_support::{certificate_authority, leaf_certificate};

fn text(value: &str) -> BoundedText {
    BoundedText::copy_from_str(value, ByteLimit::new(128)).unwrap()
}

fn bytes(value: &[u8]) -> BoundedBytes {
    BoundedBytes::copy_from_slice(value, ByteLimit::new(128)).unwrap()
}

fn opaque<T>(
    low: u64,
    build: impl FnOnce(IdParts) -> Result<T, tablerock_core::IdDecodeError>,
) -> T {
    build(IdParts::new(0, low).unwrap()).unwrap()
}

fn authorized_ttl_plan(
    logical_database: u32,
    key: &[u8],
    changes: Vec<MutationChange>,
) -> AuthorizedMutationPlan {
    let scope = OperationScope::new(
        opaque(801, ProfileId::from_parts),
        opaque(802, SessionId::from_parts),
        opaque(803, ContextId::from_parts),
    );
    let token_id = opaque(805, ReviewTokenId::from_parts);
    let reviewed = MutationPlan::new(
        opaque(804, MutationId::from_parts),
        scope,
        Revision::INITIAL,
        MutationTarget::RedisKey {
            logical_database,
            key: bytes(key),
        },
        changes,
        MutationPlanLimits::new(8, 1, 256, 256, 1_000).unwrap(),
    )
    .unwrap()
    .review(token_id, 100, 200)
    .unwrap();
    let mut registry = MutationReviewRegistry::new(1).unwrap();
    registry.insert(reviewed, 100).unwrap();
    registry
        .authorize(token_id, 150, scope, Revision::INITIAL)
        .unwrap()
}

fn identity() -> PageIdentity {
    support::identity(Engine::Redis, 2)
}

async fn raw_tls_admin_connection(
    port: u16,
    protocol: RedisProtocol,
    fixture: &RedisTlsFixture,
    use_client_identity: bool,
) -> redis::aio::MultiplexedConnection {
    let protocol = match protocol {
        RedisProtocol::Resp2 => ProtocolVersion::RESP2,
        RedisProtocol::Resp3 => ProtocolVersion::RESP3,
    };
    let info = ConnectionAddr::TcpTls {
        host: "127.0.0.1".to_owned(),
        port,
        insecure: false,
        tls_params: None,
    }
    .into_connection_info()
    .unwrap()
    .set_redis_settings(
        RedisConnectionInfo::default()
            .set_protocol(protocol)
            .set_username(REDIS_ADMIN_USERNAME)
            .set_password(REDIS_ADMIN_PASSWORD),
    );
    let client = Client::build_with_tls(
        info,
        TlsCertificates {
            client_tls: use_client_identity.then(|| ClientTlsConfig {
                client_cert: fixture.client_certificate_pem.as_bytes().to_vec(),
                client_key: fixture.client_private_key_pem.as_bytes().to_vec(),
            }),
            root_cert: Some(fixture.ca_pem.as_bytes().to_vec()),
        },
    )
    .unwrap();
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if let Ok(connection) = client.get_multiplexed_async_connection().await {
                return connection;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("TLS Redis fixture accepts connections within fifteen seconds")
}

async fn connect_session_until_ready(
    config: &RedisConnectConfig,
    security: RedisConnectionSecurity<'_>,
) -> RedisSession {
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            match RedisSession::connect(config, security).await {
                Ok(session) => return session,
                Err(
                    tablerock_engine::RedisError::Connect
                    | tablerock_engine::RedisError::Connection
                    | tablerock_engine::RedisError::Timeout,
                ) => {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
                Err(error) => panic!("Redis fixture rejected a valid connection: {error}"),
            }
        }
    })
    .await
    .expect("Redis fixture accepts an adapter connection within fifteen seconds")
}

async fn start_tls_redis(
    tag: &str,
    fixture: &RedisTlsFixture,
    require_client_identity: bool,
    host_port: Option<u16>,
) -> ContainerAsync<GenericImage> {
    start_tls_redis_with_acl(
        tag,
        fixture,
        require_client_identity,
        host_port,
        redis_acl_file(),
    )
    .await
}

async fn start_tls_redis_with_acl(
    tag: &str,
    fixture: &RedisTlsFixture,
    require_client_identity: bool,
    host_port: Option<u16>,
    acl_file: Vec<u8>,
) -> ContainerAsync<GenericImage> {
    let auth_clients = if require_client_identity { "yes" } else { "no" };
    let request = GenericImage::new("redis", tag)
        .with_exposed_port(6379.tcp())
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
        .with_copy_to(
            CopyTargetOptions::new("/tablerock-tls/server.crt").with_mode(0o644),
            CopyDataSource::Data(fixture.server_certificate_pem.as_bytes().to_vec()),
        )
        .with_copy_to(
            CopyTargetOptions::new("/tablerock-tls/server.key").with_mode(0o600),
            CopyDataSource::Data(fixture.server_private_key_pem.as_bytes().to_vec()),
        )
        .with_copy_to(
            CopyTargetOptions::new("/tablerock-tls/ca.crt").with_mode(0o644),
            CopyDataSource::Data(fixture.ca_pem.as_bytes().to_vec()),
        )
        .with_copy_to(
            CopyTargetOptions::new("/tablerock-tls/users.acl").with_mode(0o600),
            CopyDataSource::Data(acl_file),
        )
        .with_cmd([
            "sh".to_owned(),
            "-c".to_owned(),
            format!(
                "chown -R redis:redis /tablerock-tls && exec setpriv --reuid redis --regid redis --clear-groups redis-server --port 0 --tls-port 6379 --tls-cert-file /tablerock-tls/server.crt --tls-key-file /tablerock-tls/server.key --tls-ca-cert-file /tablerock-tls/ca.crt --tls-auth-clients {auth_clients} --acl-pubsub-default resetchannels --aclfile /tablerock-tls/users.acl"
            ),
        ]);
    let request = match host_port {
        Some(port) => request.with_mapped_port(port, 6379.tcp()),
        None => request,
    };
    request.start().await.unwrap()
}

#[tokio::test]
async fn scans_binary_keys_and_values_across_supported_redis_matrix() {
    for tag in [
        "7.4.9-alpine@sha256:6ab0b6e7381779332f97b8ca76193e45b0756f38d4c0dcda72dbb3c32061ab99",
        "8.8.0-alpine@sha256:9d317178eceac8454a2284a9e6df2466b93c745529947f0cd42a0fa9609d7005",
    ] {
        verify_version(tag).await;
    }
}

#[tokio::test]
async fn scan_families_preserve_full_iteration_guarantees_during_mutation() {
    for tag in [
        "7.4.9-alpine@sha256:6ab0b6e7381779332f97b8ca76193e45b0756f38d4c0dcda72dbb3c32061ab99",
        "8.8.0-alpine@sha256:9d317178eceac8454a2284a9e6df2466b93c745529947f0cd42a0fa9609d7005",
    ] {
        verify_scan_mutation_version(tag).await;
    }
}

#[tokio::test]
async fn bounds_response_timeouts_and_reconnects_future_reads() {
    for tag in [
        "7.4.9-alpine@sha256:6ab0b6e7381779332f97b8ca76193e45b0756f38d4c0dcda72dbb3c32061ab99",
        "8.8.0-alpine@sha256:9d317178eceac8454a2284a9e6df2466b93c745529947f0cd42a0fa9609d7005",
    ] {
        verify_timeout_reconnect_version(tag).await;
    }
}

#[tokio::test]
async fn resubscribes_with_visible_gap_after_redis_restart() {
    for tag in [
        "7.4.9-alpine@sha256:6ab0b6e7381779332f97b8ca76193e45b0756f38d4c0dcda72dbb3c32061ab99",
        "8.8.0-alpine@sha256:9d317178eceac8454a2284a9e6df2466b93c745529947f0cd42a0fa9609d7005",
    ] {
        let policy = RedisRuntimePolicy::new(
            Duration::from_millis(250),
            Duration::from_millis(500),
            32,
            Duration::from_millis(100),
            Duration::from_millis(500),
        )
        .unwrap();

        for protocol in [RedisProtocol::Resp2, RedisProtocol::Resp3] {
            for kind in [
                RedisSubscriptionKind::Channel,
                RedisSubscriptionKind::Pattern,
            ] {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                let port = listener.local_addr().unwrap().port();
                drop(listener);
                let container = GenericImage::new("redis", tag)
                    .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
                    .with_mapped_port(port, 6379.tcp())
                    .start()
                    .await
                    .unwrap();
                let session = connect_session_until_ready(
                    &RedisConnectConfig::new(
                        text("127.0.0.1"),
                        port,
                        0,
                        protocol,
                        RedisTlsMode::Disable,
                    )
                    .with_runtime_policy(policy),
                    RedisConnectionSecurity::new(),
                )
                .await;
                let (selector, channel, columns) = match kind {
                    RedisSubscriptionKind::Channel => {
                        (bytes(&[0, 42, 255]), bytes(&[0, 42, 255]), 2)
                    }
                    RedisSubscriptionKind::Pattern => {
                        (bytes(&[0, 42, b'*']), bytes(&[0, 42, 255]), 3)
                    }
                };
                let options =
                    RedisSubscriptionOptions::new(PageLimits::new(1, columns, 192, 64), 64, 4);
                let mut subscription = match kind {
                    RedisSubscriptionKind::Channel => {
                        session.subscribe(selector.clone(), options).await
                    }
                    RedisSubscriptionKind::Pattern => {
                        session.psubscribe(selector.clone(), options).await
                    }
                }
                .unwrap();

                drop(container);
                tokio::time::sleep(Duration::from_millis(250)).await;
                let replacement = GenericImage::new("redis", tag)
                    .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
                    .with_mapped_port(port, 6379.tcp())
                    .start()
                    .await
                    .unwrap();

                let mut publisher = raw_connection_in_database(port, protocol, 0).await;
                tokio::time::timeout(Duration::from_secs(5), async {
                    loop {
                        let receivers: usize = redis::cmd("PUBLISH")
                            .arg(channel.as_slice())
                            .arg(&[9_u8, 0, 9])
                            .query_async(&mut publisher)
                            .await
                            .unwrap();
                        if receivers == 1 {
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                })
                .await
                .expect("subscription restores within five seconds");

                let gap = tokio::time::timeout(
                    Duration::from_secs(5),
                    subscription.next_page(identity(), 0),
                )
                .await
                .expect("reconnect gap arrives")
                .unwrap()
                .unwrap();
                assert_eq!(gap.envelope().row_count(), 0);
                assert_eq!(gap.envelope().column_count(), columns);
                assert!(
                    gap.envelope()
                        .warnings()
                        .contains(PageWarning::DeliveryDiscontinuity)
                );
                let message = subscription
                    .next_page(identity(), 0)
                    .await
                    .unwrap()
                    .unwrap();
                match kind {
                    RedisSubscriptionKind::Channel => {
                        assert_eq!(message.cell(0, 0).unwrap().bytes(), channel.as_slice());
                        assert_eq!(message.cell(0, 1).unwrap().bytes(), &[9, 0, 9]);
                    }
                    RedisSubscriptionKind::Pattern => {
                        assert_eq!(message.cell(0, 0).unwrap().bytes(), selector.as_slice());
                        assert_eq!(message.cell(0, 1).unwrap().bytes(), channel.as_slice());
                        assert_eq!(message.cell(0, 2).unwrap().bytes(), &[9, 0, 9]);
                    }
                }
                drop(replacement);
                tokio::time::sleep(Duration::from_millis(100)).await;
                let started = tokio::time::Instant::now();
                assert_eq!(
                    session.dispatch_cancel().await.unwrap(),
                    tablerock_engine::RedisCancelDispatch::RequestSent
                );
                assert_eq!(
                    tokio::time::timeout(
                        Duration::from_secs(1),
                        subscription.next_page(identity(), 1)
                    )
                    .await
                    .expect("reconnect cancellation terminates promptly"),
                    Err(tablerock_engine::RedisError::ClientCancelled)
                );
                assert!(started.elapsed() < Duration::from_secs(1));
            }
        }
    }
}

#[tokio::test]
async fn resubscribes_with_visible_gap_after_tls_redis_restart() {
    for tag in [
        "7.4.9-alpine@sha256:6ab0b6e7381779332f97b8ca76193e45b0756f38d4c0dcda72dbb3c32061ab99",
        "8.8.0-alpine@sha256:9d317178eceac8454a2284a9e6df2466b93c745529947f0cd42a0fa9609d7005",
    ] {
        for protocol in [RedisProtocol::Resp2, RedisProtocol::Resp3] {
            for require_client_identity in [false, true] {
                for kind in [
                    RedisSubscriptionKind::Channel,
                    RedisSubscriptionKind::Pattern,
                ] {
                    verify_tls_subscription_restart(tag, protocol, require_client_identity, kind)
                        .await;
                }
            }
        }
    }
}

async fn verify_tls_subscription_restart(
    tag: &str,
    protocol: RedisProtocol,
    require_client_identity: bool,
    kind: RedisSubscriptionKind,
) {
    let fixture = RedisTlsFixture::generate();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let container = start_tls_redis(tag, &fixture, require_client_identity, Some(port)).await;
    let policy = RedisRuntimePolicy::new(
        Duration::from_secs(1),
        Duration::from_secs(1),
        32,
        Duration::from_millis(100),
        Duration::from_millis(500),
    )
    .unwrap();
    let config =
        RedisConnectConfig::new(text("127.0.0.1"), port, 0, protocol, RedisTlsMode::Require)
            .with_runtime_policy(policy);
    let credentials = RedisCredentials::new(Some(REDIS_TEST_USERNAME), REDIS_TEST_PASSWORD);
    let mut tls = RedisTlsMaterial::custom_roots(fixture.ca_pem.as_bytes());
    if require_client_identity {
        tls = tls.with_client_identity(RedisClientIdentity::new(
            fixture.client_certificate_pem.as_bytes(),
            fixture.client_private_key_pem.as_bytes(),
        ));
    }
    let session = connect_session_until_ready(
        &config,
        RedisConnectionSecurity::new()
            .with_credentials(credentials)
            .with_tls(tls),
    )
    .await;
    let (selector, channel, columns) = match kind {
        RedisSubscriptionKind::Channel => (bytes(&[5, 0, 255]), bytes(&[5, 0, 255]), 2),
        RedisSubscriptionKind::Pattern => (bytes(&[5, 0, b'*']), bytes(&[5, 0, 255]), 3),
    };
    let options = RedisSubscriptionOptions::new(PageLimits::new(1, columns, 192, 64), 64, 4);
    let mut subscription = match kind {
        RedisSubscriptionKind::Channel => session.subscribe(selector.clone(), options).await,
        RedisSubscriptionKind::Pattern => session.psubscribe(selector.clone(), options).await,
    }
    .unwrap();

    drop(container);
    tokio::time::sleep(Duration::from_millis(250)).await;
    let replacement = start_tls_redis(tag, &fixture, require_client_identity, Some(port)).await;
    let mut publisher =
        raw_tls_admin_connection(port, protocol, &fixture, require_client_identity).await;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let receivers: usize = redis::cmd("PUBLISH")
                .arg(channel.as_slice())
                .arg(&[8_u8, 0, 8])
                .query_async(&mut publisher)
                .await
                .unwrap();
            if receivers == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("TLS subscription restores within five seconds");

    let gap = tokio::time::timeout(
        Duration::from_secs(5),
        subscription.next_page(identity(), 0),
    )
    .await
    .expect("TLS reconnect gap arrives")
    .unwrap()
    .unwrap();
    assert_eq!(gap.envelope().row_count(), 0);
    assert_eq!(gap.envelope().column_count(), columns);
    assert!(
        gap.envelope()
            .warnings()
            .contains(PageWarning::DeliveryDiscontinuity)
    );
    let message = subscription
        .next_page(identity(), 0)
        .await
        .unwrap()
        .unwrap();
    match kind {
        RedisSubscriptionKind::Channel => {
            assert_eq!(message.cell(0, 0).unwrap().bytes(), channel.as_slice());
            assert_eq!(message.cell(0, 1).unwrap().bytes(), &[8, 0, 8]);
        }
        RedisSubscriptionKind::Pattern => {
            assert_eq!(message.cell(0, 0).unwrap().bytes(), selector.as_slice());
            assert_eq!(message.cell(0, 1).unwrap().bytes(), channel.as_slice());
            assert_eq!(message.cell(0, 2).unwrap().bytes(), &[8, 0, 8]);
        }
    }
    assert_eq!(
        session.dispatch_cancel().await.unwrap(),
        tablerock_engine::RedisCancelDispatch::RequestSent
    );
    assert_eq!(
        tokio::time::timeout(
            Duration::from_secs(1),
            subscription.next_page(identity(), 1)
        )
        .await
        .expect("TLS reconnect cancellation terminates promptly"),
        Err(tablerock_engine::RedisError::ClientCancelled)
    );
    drop(replacement);
}

#[tokio::test]
async fn rejects_untrusted_or_recredentialed_tls_pubsub_replacement() {
    for tag in [
        "7.4.9-alpine@sha256:6ab0b6e7381779332f97b8ca76193e45b0756f38d4c0dcda72dbb3c32061ab99",
        "8.8.0-alpine@sha256:9d317178eceac8454a2284a9e6df2466b93c745529947f0cd42a0fa9609d7005",
    ] {
        for protocol in [RedisProtocol::Resp2, RedisProtocol::Resp3] {
            for require_client_identity in [false, true] {
                for kind in [
                    RedisSubscriptionKind::Channel,
                    RedisSubscriptionKind::Pattern,
                ] {
                    for invalid_trust in [false, true] {
                        verify_rejected_tls_subscription_replacement(
                            tag,
                            protocol,
                            require_client_identity,
                            kind,
                            invalid_trust,
                        )
                        .await;
                    }
                }
            }
        }
    }
}

async fn verify_rejected_tls_subscription_replacement(
    tag: &str,
    protocol: RedisProtocol,
    require_client_identity: bool,
    kind: RedisSubscriptionKind,
    invalid_trust: bool,
) {
    let fixture = RedisTlsFixture::generate();
    let invalid_fixture = RedisTlsFixture::generate();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let container = start_tls_redis(tag, &fixture, require_client_identity, Some(port)).await;
    let policy = RedisRuntimePolicy::new(
        Duration::from_secs(1),
        Duration::from_secs(1),
        32,
        Duration::from_millis(250),
        Duration::from_millis(500),
    )
    .unwrap();
    let config =
        RedisConnectConfig::new(text("127.0.0.1"), port, 0, protocol, RedisTlsMode::Require)
            .with_runtime_policy(policy);
    let credentials = RedisCredentials::new(Some(REDIS_TEST_USERNAME), REDIS_TEST_PASSWORD);
    let mut tls = RedisTlsMaterial::custom_roots(fixture.ca_pem.as_bytes());
    if require_client_identity {
        tls = tls.with_client_identity(RedisClientIdentity::new(
            fixture.client_certificate_pem.as_bytes(),
            fixture.client_private_key_pem.as_bytes(),
        ));
    }
    let session = connect_session_until_ready(
        &config,
        RedisConnectionSecurity::new()
            .with_credentials(credentials)
            .with_tls(tls),
    )
    .await;
    let (selector, columns) = match kind {
        RedisSubscriptionKind::Channel => (bytes(b"replacement:channel"), 2),
        RedisSubscriptionKind::Pattern => (bytes(b"replacement:*"), 3),
    };
    let options = RedisSubscriptionOptions::new(PageLimits::new(1, columns, 192, 64), 64, 4);
    let mut subscription = match kind {
        RedisSubscriptionKind::Channel => session.subscribe(selector.clone(), options).await,
        RedisSubscriptionKind::Pattern => session.psubscribe(selector.clone(), options).await,
    }
    .unwrap();
    let mut observer =
        raw_tls_admin_connection(port, protocol, &fixture, require_client_identity).await;
    match kind {
        RedisSubscriptionKind::Channel => {
            let counts: Vec<(Vec<u8>, u64)> = redis::cmd("PUBSUB")
                .arg("NUMSUB")
                .arg(selector.as_slice())
                .query_async(&mut observer)
                .await
                .unwrap();
            assert_eq!(counts, vec![(selector.as_slice().to_vec(), 1)]);
        }
        RedisSubscriptionKind::Pattern => {
            let patterns: u64 = redis::cmd("PUBSUB")
                .arg("NUMPAT")
                .query_async(&mut observer)
                .await
                .unwrap();
            assert_eq!(patterns, 1);
        }
    }
    drop(observer);
    drop(container);
    tokio::time::sleep(Duration::from_millis(100)).await;

    let replacement_fixture = if invalid_trust {
        &invalid_fixture
    } else {
        &fixture
    };
    let replacement_acl = if invalid_trust {
        redis_acl_file()
    } else {
        redis_rotated_test_acl_file()
    };
    let replacement = start_tls_redis_with_acl(
        tag,
        replacement_fixture,
        require_client_identity,
        Some(port),
        replacement_acl,
    )
    .await;
    let replacement_ready =
        raw_tls_admin_connection(port, protocol, replacement_fixture, require_client_identity)
            .await;
    drop(replacement_ready);
    let expected = if invalid_trust {
        tablerock_engine::RedisError::Connect
    } else {
        tablerock_engine::RedisError::Authentication
    };
    assert_eq!(
        tokio::time::timeout(
            Duration::from_secs(60),
            subscription.next_page(identity(), 0)
        )
        .await
        .expect("rejected TLS replacement terminates within policy deadline"),
        Err(expected)
    );
    drop(replacement);
}

#[tokio::test]
async fn bounds_connection_handshake_timeout() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        let (_connection, _) = listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_secs(2)).await;
    });
    let policy = RedisRuntimePolicy::new(
        Duration::from_millis(100),
        Duration::from_millis(100),
        1,
        Duration::from_millis(1),
        Duration::from_millis(1),
    )
    .unwrap();
    let started = tokio::time::Instant::now();
    let result = RedisSession::connect(
        &RedisConnectConfig::new(
            text("127.0.0.1"),
            port,
            0,
            RedisProtocol::Resp3,
            RedisTlsMode::Disable,
        )
        .with_runtime_policy(policy),
        RedisConnectionSecurity::new(),
    )
    .await;
    assert!(matches!(result, Err(tablerock_engine::RedisError::Timeout)));
    assert!(started.elapsed() < Duration::from_secs(1));
    server.abort();
}

#[tokio::test]
async fn verifies_tls_acl_authentication_and_optional_client_identity() {
    for tag in [
        "7.4.9-alpine@sha256:6ab0b6e7381779332f97b8ca76193e45b0756f38d4c0dcda72dbb3c32061ab99",
        "8.8.0-alpine@sha256:9d317178eceac8454a2284a9e6df2466b93c745529947f0cd42a0fa9609d7005",
    ] {
        for protocol in [RedisProtocol::Resp2, RedisProtocol::Resp3] {
            verify_tls_auth_version(tag, protocol, false).await;
            verify_tls_auth_version(tag, protocol, true).await;
        }
    }
}

async fn verify_tls_auth_version(
    tag: &str,
    protocol: RedisProtocol,
    require_client_identity: bool,
) {
    let fixture = RedisTlsFixture::generate();
    let container = start_tls_redis(tag, &fixture, require_client_identity, None).await;
    let port = container.get_host_port_ipv4(6379.tcp()).await.unwrap();
    let config =
        RedisConnectConfig::new(text("127.0.0.1"), port, 1, protocol, RedisTlsMode::Require)
            .with_runtime_policy(
                RedisRuntimePolicy::new(
                    Duration::from_secs(2),
                    Duration::from_secs(2),
                    2,
                    Duration::from_millis(10),
                    Duration::from_millis(50),
                )
                .unwrap(),
            );
    let credentials = RedisCredentials::new(Some(REDIS_TEST_USERNAME), REDIS_TEST_PASSWORD);
    let mut tls = RedisTlsMaterial::custom_roots(fixture.ca_pem.as_bytes());
    if require_client_identity {
        tls = tls.with_client_identity(RedisClientIdentity::new(
            fixture.client_certificate_pem.as_bytes(),
            fixture.client_private_key_pem.as_bytes(),
        ));
    }
    let security = RedisConnectionSecurity::new()
        .with_credentials(credentials)
        .with_tls(tls);
    let session = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            match RedisSession::connect(&config, security).await {
                Ok(session) => break session,
                Err(tablerock_engine::RedisError::Connect) => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(error) => panic!(
                    "Redis TLS connect failed for {tag}, {protocol:?}, mTLS={require_client_identity}: {error:?}"
                ),
            }
        }
    })
    .await
    .expect("Redis TLS fixture becomes reachable");
    let original_client_id = session.observed_client_id().await.unwrap();
    let mut admin =
        raw_tls_admin_connection(port, protocol, &fixture, require_client_identity).await;
    let _: () = redis::cmd("SELECT")
        .arg(1)
        .query_async(&mut admin)
        .await
        .unwrap();
    let _: () = redis::cmd("SET")
        .arg(b"tls-reconnect-database-one")
        .arg(&[0_u8, 255])
        .query_async(&mut admin)
        .await
        .unwrap();
    let killed: u64 = redis::cmd("CLIENT")
        .arg("KILL")
        .arg("ID")
        .arg(original_client_id)
        .query_async(&mut admin)
        .await
        .unwrap();
    assert_eq!(killed, 1);
    let reconnected_client_id = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match session.observed_client_id().await {
                Ok(client_id) if client_id != original_client_id => break client_id,
                Ok(_) | Err(tablerock_engine::RedisError::Connection) => {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                Err(error) => panic!("TLS reconnect failed: {error:?}"),
            }
        }
    })
    .await
    .expect("TLS-authenticated future call reconnects");
    assert_ne!(reconnected_client_id, original_client_id);
    let restored = session
        .read_binary(&bytes(b"tls-reconnect-database-one"), 16)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        restored.as_ref(),
        tablerock_core::ValueRef::Binary {
            value: [0, 255],
            truncation: Truncation::Complete,
        }
    ));

    let mut blocking = session
        .blocking_pop(
            bytes(b"tablerock-tls-cancellation"),
            PageLimits::new(1, 2, 256, 128),
            128,
        )
        .await
        .unwrap();
    assert_eq!(
        session.dispatch_cancel().await.unwrap(),
        tablerock_engine::RedisCancelDispatch::RequestSent
    );
    assert_eq!(
        blocking.next_page(identity(), 0).await,
        Err(tablerock_engine::RedisError::ServerCancelled)
    );

    for kind in [
        RedisSubscriptionKind::Channel,
        RedisSubscriptionKind::Pattern,
    ] {
        let (selector, channel, columns) = match kind {
            RedisSubscriptionKind::Channel => (bytes(&[7, 0, 255]), bytes(&[7, 0, 255]), 2),
            RedisSubscriptionKind::Pattern => (bytes(&[7, 0, b'*']), bytes(&[7, 0, 255]), 3),
        };
        let options = RedisSubscriptionOptions::new(PageLimits::new(1, columns, 192, 64), 64, 2);
        let mut subscription = match kind {
            RedisSubscriptionKind::Channel => session.subscribe(selector.clone(), options).await,
            RedisSubscriptionKind::Pattern => session.psubscribe(selector.clone(), options).await,
        }
        .unwrap();
        let receivers: usize = redis::cmd("PUBLISH")
            .arg(channel.as_slice())
            .arg(&[3_u8, 0, 4])
            .query_async(&mut admin)
            .await
            .unwrap();
        assert_eq!(receivers, 1);
        let page = tokio::time::timeout(
            Duration::from_secs(5),
            subscription.next_page(identity(), 0),
        )
        .await
        .expect("TLS Pub/Sub delivers within five seconds")
        .unwrap()
        .unwrap();
        assert_eq!(page.envelope().column_count(), columns);
        match kind {
            RedisSubscriptionKind::Channel => {
                assert_eq!(page.cell(0, 0).unwrap().bytes(), channel.as_slice());
                assert_eq!(page.cell(0, 1).unwrap().bytes(), &[3, 0, 4]);
            }
            RedisSubscriptionKind::Pattern => {
                assert_eq!(page.cell(0, 0).unwrap().bytes(), selector.as_slice());
                assert_eq!(page.cell(0, 1).unwrap().bytes(), channel.as_slice());
                assert_eq!(page.cell(0, 2).unwrap().bytes(), &[3, 0, 4]);
            }
        }
        assert_eq!(
            session.dispatch_cancel().await.unwrap(),
            tablerock_engine::RedisCancelDispatch::RequestSent
        );
        assert_eq!(
            tokio::time::timeout(
                Duration::from_secs(5),
                subscription.next_page(identity(), 1)
            )
            .await
            .expect("TLS Pub/Sub cancellation completes within five seconds"),
            Err(tablerock_engine::RedisError::ClientCancelled)
        );
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let removed = match kind {
                    RedisSubscriptionKind::Channel => {
                        let counts: Vec<(Vec<u8>, u64)> = redis::cmd("PUBSUB")
                            .arg("NUMSUB")
                            .arg(selector.as_slice())
                            .query_async(&mut admin)
                            .await
                            .unwrap();
                        counts == vec![(selector.as_slice().to_vec(), 0)]
                    }
                    RedisSubscriptionKind::Pattern => {
                        redis::cmd("PUBSUB")
                            .arg("NUMPAT")
                            .query_async::<u64>(&mut admin)
                            .await
                            .unwrap()
                            == 0
                    }
                };
                if removed {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("TLS Pub/Sub teardown is server-visible within five seconds");
    }

    let dry_run_denial: String = redis::cmd("ACL")
        .arg("DRYRUN")
        .arg(REDIS_RESTRICTED_USERNAME)
        .arg("SUBSCRIBE")
        .arg("denied:channel")
        .query_async(&mut admin)
        .await
        .unwrap();
    assert!(
        dry_run_denial.contains("no permissions to access") && dry_run_denial.contains("channel"),
        "unexpected ACL DRYRUN denial: {dry_run_denial:?}"
    );

    if require_client_identity {
        let without_identity = RedisConnectionSecurity::new()
            .with_credentials(credentials)
            .with_tls(RedisTlsMaterial::custom_roots(fixture.ca_pem.as_bytes()));
        assert!(matches!(
            RedisSession::connect(&config, without_identity).await,
            Err(tablerock_engine::RedisError::Connect)
        ));
    }

    let mut wrong_password_tls = RedisTlsMaterial::custom_roots(fixture.ca_pem.as_bytes());
    if require_client_identity {
        wrong_password_tls = wrong_password_tls.with_client_identity(RedisClientIdentity::new(
            fixture.client_certificate_pem.as_bytes(),
            fixture.client_private_key_pem.as_bytes(),
        ));
    }
    let wrong_password = RedisConnectionSecurity::new()
        .with_credentials(RedisCredentials::new(
            Some(REDIS_TEST_USERNAME),
            "wrong-synthetic-password",
        ))
        .with_tls(wrong_password_tls);
    let authentication_started = tokio::time::Instant::now();
    assert_eq!(
        RedisSession::connect(&config, wrong_password)
            .await
            .map(|_| ()),
        Err(tablerock_engine::RedisError::Authentication)
    );
    assert!(authentication_started.elapsed() < Duration::from_secs(1));

    let mut wrong_ca_tls = RedisTlsMaterial::custom_roots(fixture.wrong_ca_pem.as_bytes());
    if require_client_identity {
        wrong_ca_tls = wrong_ca_tls.with_client_identity(RedisClientIdentity::new(
            fixture.client_certificate_pem.as_bytes(),
            fixture.client_private_key_pem.as_bytes(),
        ));
    }
    let wrong_ca = RedisConnectionSecurity::new()
        .with_credentials(credentials)
        .with_tls(wrong_ca_tls);
    assert!(matches!(
        RedisSession::connect(&config, wrong_ca).await,
        Err(tablerock_engine::RedisError::Connect)
    ));

    let hostname_mismatch =
        RedisConnectConfig::new(text("localhost"), port, 1, protocol, RedisTlsMode::Require);
    let mut hostname_tls = RedisTlsMaterial::custom_roots(fixture.ca_pem.as_bytes());
    if require_client_identity {
        hostname_tls = hostname_tls.with_client_identity(RedisClientIdentity::new(
            fixture.client_certificate_pem.as_bytes(),
            fixture.client_private_key_pem.as_bytes(),
        ));
    }
    assert!(matches!(
        RedisSession::connect(
            &hostname_mismatch,
            RedisConnectionSecurity::new()
                .with_credentials(credentials)
                .with_tls(hostname_tls),
        )
        .await,
        Err(tablerock_engine::RedisError::Connect)
    ));

    let plaintext =
        RedisConnectConfig::new(text("127.0.0.1"), port, 0, protocol, RedisTlsMode::Disable)
            .with_runtime_policy(
                RedisRuntimePolicy::new(
                    Duration::from_millis(250),
                    Duration::from_millis(250),
                    1,
                    Duration::from_millis(1),
                    Duration::from_millis(1),
                )
                .unwrap(),
            );
    assert!(
        RedisSession::connect(&plaintext, RedisConnectionSecurity::new())
            .await
            .is_err()
    );

    let revoked_channel = bytes(b"revocation:channel");
    let mut revoked_subscription = session
        .subscribe(
            revoked_channel.clone(),
            RedisSubscriptionOptions::new(PageLimits::new(1, 2, 192, 64), 64, 2),
        )
        .await
        .unwrap();
    let counts: Vec<(Vec<u8>, u64)> = redis::cmd("PUBSUB")
        .arg("NUMSUB")
        .arg(revoked_channel.as_slice())
        .query_async(&mut admin)
        .await
        .unwrap();
    assert_eq!(counts, vec![(revoked_channel.as_slice().to_vec(), 1)]);

    let pattern_credentials =
        RedisCredentials::new(Some(REDIS_PATTERN_USERNAME), REDIS_PATTERN_PASSWORD);
    let mut pattern_tls = RedisTlsMaterial::custom_roots(fixture.ca_pem.as_bytes());
    if require_client_identity {
        pattern_tls = pattern_tls.with_client_identity(RedisClientIdentity::new(
            fixture.client_certificate_pem.as_bytes(),
            fixture.client_private_key_pem.as_bytes(),
        ));
    }
    let pattern_session = RedisSession::connect(
        &config,
        RedisConnectionSecurity::new()
            .with_credentials(pattern_credentials)
            .with_tls(pattern_tls),
    )
    .await
    .unwrap();
    let revoked_pattern = bytes(b"revocation:*");
    let mut revoked_pattern_subscription = pattern_session
        .psubscribe(
            revoked_pattern,
            RedisSubscriptionOptions::new(PageLimits::new(1, 3, 192, 64), 64, 2),
        )
        .await
        .unwrap();
    let patterns: u64 = redis::cmd("PUBSUB")
        .arg("NUMPAT")
        .query_async(&mut admin)
        .await
        .unwrap();
    assert_eq!(patterns, 1);

    let _: () = redis::cmd("ACL")
        .arg("SETUSER")
        .arg(REDIS_TEST_USERNAME)
        .arg("resetpass")
        .arg(">synthetic-rotated-password")
        .query_async(&mut admin)
        .await
        .unwrap();
    let _: () = redis::cmd("ACL")
        .arg("SETUSER")
        .arg(REDIS_PATTERN_USERNAME)
        .arg("resetpass")
        .arg(">synthetic-rotated-pattern-password")
        .query_async(&mut admin)
        .await
        .unwrap();
    let (killed, pattern_killed): (u64, u64) = redis::pipe()
        .cmd("CLIENT")
        .arg("KILL")
        .arg("USER")
        .arg(REDIS_TEST_USERNAME)
        .cmd("CLIENT")
        .arg("KILL")
        .arg("USER")
        .arg(REDIS_PATTERN_USERNAME)
        .query_async(&mut admin)
        .await
        .unwrap();
    assert!(killed >= 1);
    assert!(pattern_killed >= 1);
    assert_eq!(
        tokio::time::timeout(
            Duration::from_secs(5),
            revoked_subscription.next_page(identity(), 0)
        )
        .await
        .expect("active subscription revocation stops within five seconds"),
        Err(tablerock_engine::RedisError::Authentication)
    );
    assert_eq!(
        tokio::time::timeout(
            Duration::from_secs(5),
            revoked_pattern_subscription.next_page(identity(), 0)
        )
        .await
        .expect("active pattern revocation stops within five seconds"),
        Err(tablerock_engine::RedisError::Authentication)
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match session.observed_client_id().await {
                Err(tablerock_engine::RedisError::Authentication) => break,
                Err(tablerock_engine::RedisError::Connection) => {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                result => panic!("unexpected result after live ACL revocation: {result:?}"),
            }
        }
    })
    .await
    .expect("live ACL revocation stops reconnect within five seconds");
}

async fn verify_timeout_reconnect_version(tag: &str) {
    let container = GenericImage::new("redis", tag)
        .with_exposed_port(6379.tcp())
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(6379.tcp()).await.unwrap();
    let policy = RedisRuntimePolicy::new(
        Duration::from_millis(250),
        Duration::from_millis(100),
        8,
        Duration::from_millis(10),
        Duration::from_millis(100),
    )
    .unwrap();

    for protocol in [RedisProtocol::Resp2, RedisProtocol::Resp3] {
        let mut fixture = raw_connection_in_database(port, protocol, 1).await;
        let _: () = redis::cmd("SET")
            .arg(b"reconnect-key")
            .arg(&[0_u8, 255])
            .query_async(&mut fixture)
            .await
            .unwrap();
        let session = RedisSession::connect(
            &RedisConnectConfig::new(text("127.0.0.1"), port, 1, protocol, RedisTlsMode::Disable)
                .with_runtime_policy(policy),
            RedisConnectionSecurity::new(),
        )
        .await
        .unwrap();

        let _: () = redis::cmd("CLIENT")
            .arg("PAUSE")
            .arg(300_u64)
            .arg("ALL")
            .query_async(&mut fixture)
            .await
            .unwrap();
        assert_eq!(
            session.read_binary(&bytes(b"reconnect-key"), 16).await,
            Err(tablerock_engine::RedisError::Timeout),
            "response timeout {tag} {protocol:?}"
        );
        tokio::time::sleep(Duration::from_millis(350)).await;
        assert!(matches!(
            session
                .read_binary(&bytes(b"reconnect-key"), 16)
                .await
                .unwrap()
                .unwrap()
                .as_ref(),
            tablerock_core::ValueRef::Binary {
                value: [0, 255],
                truncation: Truncation::Complete,
            }
        ));

        let client_id = session.observed_client_id().await.unwrap();
        let killed: u64 = redis::cmd("CLIENT")
            .arg("KILL")
            .arg("ID")
            .arg(client_id)
            .query_async(&mut fixture)
            .await
            .unwrap();
        assert_eq!(killed, 1);
        let first_post_drop = session.read_binary(&bytes(b"reconnect-key"), 16).await;
        if matches!(protocol, RedisProtocol::Resp2) {
            assert_eq!(
                first_post_drop,
                Err(tablerock_engine::RedisError::Connection)
            );
        }
        match first_post_drop {
            Ok(Some(value)) => assert!(matches!(
                value.as_ref(),
                tablerock_core::ValueRef::Binary {
                    value: [0, 255],
                    truncation: Truncation::Complete,
                }
            )),
            Err(tablerock_engine::RedisError::Connection) => {}
            other => panic!("unexpected first post-drop read: {other:?}"),
        }
        let value = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                match session.read_binary(&bytes(b"reconnect-key"), 16).await {
                    Ok(Some(value)) => break value,
                    Ok(None) | Err(_) => tokio::time::sleep(Duration::from_millis(25)).await,
                }
            }
        })
        .await
        .expect("future reads reconnect within five seconds");
        assert!(matches!(
            value.as_ref(),
            tablerock_core::ValueRef::Binary {
                value: [0, 255],
                truncation: Truncation::Complete,
            }
        ));
    }
}

async fn verify_scan_mutation_version(tag: &str) {
    let container = GenericImage::new("redis", tag)
        .with_exposed_port(6379.tcp())
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(6379.tcp()).await.unwrap();

    for protocol in [RedisProtocol::Resp2, RedisProtocol::Resp3] {
        let mut mutator = raw_connection(port, protocol).await;
        let stable = seed_scan_race(&mut mutator).await;
        let session = RedisSession::connect(
            &RedisConnectConfig::new(text("127.0.0.1"), port, 0, protocol, RedisTlsMode::Disable),
            RedisConnectionSecurity::new(),
        )
        .await
        .unwrap();

        verify_keyspace_mutation(&session, &mut mutator, &stable, protocol, tag).await;
        for (kind, key) in [
            (RedisCollectionScanKind::Hash, b"race-hash".as_slice()),
            (RedisCollectionScanKind::Set, b"race-set".as_slice()),
            (RedisCollectionScanKind::SortedSet, b"race-zset".as_slice()),
        ] {
            verify_collection_mutation(&session, &mut mutator, &stable, kind, key, protocol, tag)
                .await;
        }
    }
}

async fn raw_connection(port: u16, protocol: RedisProtocol) -> redis::aio::MultiplexedConnection {
    raw_connection_in_database(port, protocol, 0).await
}

async fn raw_connection_in_database(
    port: u16,
    protocol: RedisProtocol,
    database: i64,
) -> redis::aio::MultiplexedConnection {
    let redis = RedisConnectionInfo::default()
        .set_db(database)
        .set_protocol(match protocol {
            RedisProtocol::Resp2 => ProtocolVersion::RESP2,
            RedisProtocol::Resp3 => ProtocolVersion::RESP3,
        });
    let info = ConnectionAddr::Tcp("127.0.0.1".to_owned(), port)
        .into_connection_info()
        .unwrap()
        .set_redis_settings(redis);
    let client = redis::Client::open(info).unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(connection) = client.get_multiplexed_async_connection().await {
                return connection;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("Redis fixture accepts connections within five seconds")
}

async fn seed_scan_race(connection: &mut redis::aio::MultiplexedConnection) -> BTreeSet<Vec<u8>> {
    let mut stable = (0..600_u16)
        .map(|index| format!("stable-{index:04}").into_bytes())
        .collect::<BTreeSet<_>>();
    stable.insert(vec![0, 255]);
    let mut keys = redis::pipe();
    for member in &stable {
        keys.cmd("SET")
            .arg([b"race-key:".as_slice(), member].concat())
            .arg(1_u8);
    }
    keys.cmd("SET").arg(b"race-key:transient").arg(1_u8);
    keys.cmd("SET").arg(b"race-key:removed-before").arg(1_u8);
    let _: Vec<redis::Value> = keys.query_async(connection).await.unwrap();

    let mut hash = redis::cmd("HSET");
    hash.arg(b"race-hash");
    for member in &stable {
        hash.arg(member).arg(b"value");
    }
    hash.arg(b"transient").arg(b"value");
    hash.arg(b"removed-before").arg(b"value");
    let _: u64 = hash.query_async(connection).await.unwrap();

    let mut set = redis::cmd("SADD");
    set.arg(b"race-set");
    for member in &stable {
        set.arg(member);
    }
    set.arg(b"transient").arg(b"removed-before");
    let _: u64 = set.query_async(connection).await.unwrap();

    let mut sorted_set = redis::cmd("ZADD");
    sorted_set.arg(b"race-zset");
    for (score, member) in stable.iter().enumerate() {
        sorted_set.arg(score).arg(member);
    }
    sorted_set
        .arg(10_000_u64)
        .arg(b"transient")
        .arg(10_001_u64)
        .arg(b"removed-before");
    let _: u64 = sorted_set.query_async(connection).await.unwrap();

    let _: u64 = redis::cmd("DEL")
        .arg(b"race-key:removed-before")
        .arg(b"race-key:late")
        .query_async(connection)
        .await
        .unwrap();
    let _: u64 = redis::cmd("HDEL")
        .arg(b"race-hash")
        .arg(b"removed-before")
        .arg(b"late")
        .query_async(connection)
        .await
        .unwrap();
    let _: u64 = redis::cmd("SREM")
        .arg(b"race-set")
        .arg(b"removed-before")
        .arg(b"late")
        .query_async(connection)
        .await
        .unwrap();
    let _: u64 = redis::cmd("ZREM")
        .arg(b"race-zset")
        .arg(b"removed-before")
        .arg(b"late")
        .query_async(connection)
        .await
        .unwrap();
    stable
}

async fn verify_keyspace_mutation(
    session: &RedisSession,
    mutator: &mut redis::aio::MultiplexedConnection,
    stable: &BTreeSet<Vec<u8>>,
    protocol: RedisProtocol,
    tag: &str,
) {
    let mut stream = session
        .scan_keys(PageLimits::new(8, 1, 4_096, 64), 128, 8, 4_096)
        .unwrap();
    let first = stream.next_page(identity(), 0).await.unwrap().unwrap();
    assert_eq!(first.envelope().delivery(), PageDelivery::Partial);
    let _: u64 = redis::cmd("DEL")
        .arg(b"race-key:transient")
        .query_async(&mut *mutator)
        .await
        .unwrap();
    let _: () = redis::cmd("SET")
        .arg(b"race-key:late")
        .arg(1_u8)
        .query_async(&mut *mutator)
        .await
        .unwrap();
    let mut seen = BTreeSet::new();
    collect_first_column(&first, &mut seen);
    let mut start = u64::from(first.envelope().row_count());
    while let Some(page) = stream.next_page(identity(), start).await.unwrap() {
        collect_first_column(&page, &mut seen);
        start += u64::from(page.envelope().row_count());
    }
    for member in stable {
        assert!(
            seen.contains(&[b"race-key:".as_slice(), member].concat()),
            "stable SCAN key {member:?} {tag} {protocol:?}"
        );
    }
    assert!(!seen.contains(b"race-key:removed-before".as_slice()));
}

async fn verify_collection_mutation(
    session: &RedisSession,
    mutator: &mut redis::aio::MultiplexedConnection,
    stable: &BTreeSet<Vec<u8>>,
    kind: RedisCollectionScanKind,
    key: &[u8],
    protocol: RedisProtocol,
    tag: &str,
) {
    let mut stream = session
        .scan_collection(
            bytes(key),
            kind,
            RedisCollectionScanOptions::new(
                PageLimits::new(8, 2, 4_096, 64),
                128,
                8,
                128,
                65_536,
                4_096,
            ),
        )
        .unwrap();
    let first = stream.next_page(identity(), 0).await.unwrap().unwrap();
    assert_eq!(first.envelope().delivery(), PageDelivery::Partial);
    match kind {
        RedisCollectionScanKind::Hash => {
            let _: u64 = redis::cmd("HDEL")
                .arg(key)
                .arg(b"transient")
                .query_async(&mut *mutator)
                .await
                .unwrap();
            let _: u64 = redis::cmd("HSET")
                .arg(key)
                .arg(b"late")
                .arg(b"value")
                .query_async(&mut *mutator)
                .await
                .unwrap();
        }
        RedisCollectionScanKind::Set => {
            let _: u64 = redis::cmd("SREM")
                .arg(key)
                .arg(b"transient")
                .query_async(&mut *mutator)
                .await
                .unwrap();
            let _: u64 = redis::cmd("SADD")
                .arg(key)
                .arg(b"late")
                .query_async(&mut *mutator)
                .await
                .unwrap();
        }
        RedisCollectionScanKind::SortedSet => {
            let _: u64 = redis::cmd("ZREM")
                .arg(key)
                .arg(b"transient")
                .query_async(&mut *mutator)
                .await
                .unwrap();
            let _: u64 = redis::cmd("ZADD")
                .arg(key)
                .arg(10_002_u64)
                .arg(b"late")
                .query_async(&mut *mutator)
                .await
                .unwrap();
        }
    }
    let mut seen = BTreeSet::new();
    collect_first_column(&first, &mut seen);
    let mut start = u64::from(first.envelope().row_count());
    while let Some(page) = stream.next_page(identity(), start).await.unwrap() {
        collect_first_column(&page, &mut seen);
        start += u64::from(page.envelope().row_count());
    }
    assert!(
        stable.is_subset(&seen),
        "stable {kind:?} members {tag} {protocol:?}"
    );
    assert!(!seen.contains(b"removed-before".as_slice()));
}

fn collect_first_column(page: &ResultPage, values: &mut BTreeSet<Vec<u8>>) {
    assert_ne!(page.envelope().row_count(), 0);
    for row in 0..page.envelope().row_count() {
        values.insert(page.cell(row, 0).unwrap().bytes().to_vec());
    }
}

async fn verify_version(tag: &str) {
    let container = GenericImage::new("redis", tag)
        .with_exposed_port(6379.tcp())
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(6379.tcp()).await.unwrap();
    seed(port).await;

    for protocol in [RedisProtocol::Resp2, RedisProtocol::Resp3] {
        let session = RedisSession::connect(
            &RedisConnectConfig::new(text("127.0.0.1"), port, 0, protocol, RedisTlsMode::Disable),
            RedisConnectionSecurity::new(),
        )
        .await
        .unwrap();
        assert_eq!(session.negotiated_protocol().await.unwrap(), protocol);
        verify_pubsub_isolation(&session, port, protocol, tag).await;
        verify_pubsub_service_cancellation(port, protocol).await;
        verify_ttl_states(&session, protocol, tag).await;
        verify_ttl_mutations(&session, port, protocol, tag).await;
        verify_hash_scan(&session, protocol, tag).await;
        verify_set_scan(&session, protocol, tag).await;
        verify_sorted_set_scan(&session, protocol, tag).await;
        verify_empty_collection_scans(&session, protocol, tag).await;
        verify_pipeline_partial_failure(port, protocol, tag).await;
        verify_service_cancellation(port, protocol, tag, false).await;
        verify_service_cancellation(port, protocol, tag, true).await;
        verify_blocking_completion(port, protocol).await;

        let value = session
            .read_binary(&bytes(&[0, 255]), 3)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(value.kind(), ValueKind::Binary);
        assert!(matches!(
            value.as_ref(),
            tablerock_core::ValueRef::Binary {
                value: [1, 0, 255],
                truncation: Truncation::Truncated {
                    original_byte_len: Some(4)
                }
            }
        ));

        let driver: &dyn DriverSession = &session;
        assert_eq!(driver.engine(), Engine::Redis);
        let mut stream = driver
            .start_page_stream(DriverPageRequest::RedisKeyScan {
                limits: PageLimits::new(2, 1, 256, 64),
                max_cell_bytes: 128,
                scan_count: 2,
                max_scan_rounds: 128,
            })
            .await
            .unwrap();
        let mut found = BTreeSet::new();
        let mut start = 0_u64;
        while let Some(page) = stream.next_page(identity(), start).await.unwrap() {
            assert_ne!(page.envelope().row_count(), 0);
            for row in 0..page.envelope().row_count() {
                found.insert(page.cell(row, 0).unwrap().bytes().to_vec());
            }
            start += u64::from(page.envelope().row_count());
        }
        assert_eq!(
            found,
            BTreeSet::from([
                vec![0, 255],
                b"long-binary-key".to_vec(),
                b"plain".to_vec(),
                b"scan-hash".to_vec(),
                b"scan-set".to_vec(),
                b"scan-zset".to_vec(),
            ]),
            "Redis {tag} {protocol:?}"
        );

        let mut bounded = session
            .scan_keys(PageLimits::new(8, 1, 4, 64), 2, 8, 128)
            .unwrap();
        let page = bounded.next_page(identity(), 0).await.unwrap().unwrap();
        assert!(
            page.envelope()
                .warnings()
                .contains(PageWarning::ByteLimitReached)
        );
        let stored_bytes: usize = (0..page.envelope().row_count())
            .map(|row| page.cell(row, 0).unwrap().bytes().len())
            .sum();
        assert!(stored_bytes <= 4);

        let isolated = RedisSession::connect(
            &RedisConnectConfig::new(text("127.0.0.1"), port, 1, protocol, RedisTlsMode::Disable),
            RedisConnectionSecurity::new(),
        )
        .await
        .unwrap();
        let mut isolated_scan = isolated
            .scan_keys(PageLimits::new(8, 1, 128, 64), 128, 8, 128)
            .unwrap();
        let page = isolated_scan
            .next_page(identity(), 0)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(page.envelope().delivery(), PageDelivery::Final);
        assert_eq!(page.envelope().row_count(), 1);
        assert_eq!(page.cell(0, 0).unwrap().bytes(), b"database-one");

        drop(stream);
        drop(bounded);
        let operation_id = support::operation(40);
        let mut service = support::service(1, 2);
        service
            .submit(
                operation_id,
                support::command(41),
                Box::new(session),
                DriverPageRequest::RedisKeyScan {
                    limits: PageLimits::new(2, 1, 256, 64),
                    max_cell_bytes: 128,
                    scan_count: 2,
                    max_scan_rounds: 128,
                },
                identity(),
            )
            .await
            .unwrap();
        let mut service_keys = BTreeSet::new();
        loop {
            match service.next_update(operation_id).await.unwrap().unwrap() {
                EngineServiceUpdate::Started => {}
                EngineServiceUpdate::Page(page) => {
                    for row in 0..page.envelope().row_count() {
                        service_keys.insert(page.cell(row, 0).unwrap().bytes().to_vec());
                    }
                }
                EngineServiceUpdate::Terminal(tablerock_core::OperationOutcome::Completed) => {
                    break;
                }
                other => panic!("unexpected Redis service event: {other:?}"),
            }
        }
        assert_eq!(service_keys, found, "Redis service {tag} {protocol:?}");
    }
}

async fn verify_hash_scan(session: &RedisSession, protocol: RedisProtocol, tag: &str) {
    let driver: &dyn DriverSession = session;
    let mut stream = driver
        .start_page_stream(DriverPageRequest::RedisCollectionScan {
            key: bytes(b"scan-hash"),
            kind: RedisCollectionScanKind::Hash,
            options: RedisCollectionScanOptions::new(
                PageLimits::new(1, 2, 64, 128),
                16,
                1,
                16,
                1_024,
                128,
            ),
        })
        .await
        .unwrap();
    let mut rows = BTreeSet::new();
    let mut start = 0_u64;
    while let Some(page) = stream.next_page(identity(), start).await.unwrap() {
        assert!(page.envelope().row_count() <= 1);
        assert_eq!(page.envelope().column_count(), 2);
        for row in 0..page.envelope().row_count() {
            rows.insert((
                page.cell(row, 0).unwrap().bytes().to_vec(),
                page.cell(row, 1).unwrap().bytes().to_vec(),
            ));
        }
        start += u64::from(page.envelope().row_count());
    }
    assert_eq!(
        rows,
        BTreeSet::from([
            (vec![0, 255], vec![1, 2, 3, 4]),
            (b"field".to_vec(), b"value".to_vec()),
        ]),
        "HSCAN {tag} {protocol:?}"
    );

    let mut oversized = driver
        .start_page_stream(DriverPageRequest::RedisCollectionScan {
            key: bytes(b"scan-hash"),
            kind: RedisCollectionScanKind::Hash,
            options: RedisCollectionScanOptions::new(
                PageLimits::new(2, 2, 64, 128),
                16,
                1,
                1,
                1_024,
                128,
            ),
        })
        .await
        .unwrap();
    let error = oversized.next_page(identity(), 0).await.unwrap_err();
    assert_eq!(
        error.class(),
        AdapterFailureClass::ResourceLimit,
        "bounded HSCAN response {tag} {protocol:?}"
    );
}

async fn verify_set_scan(session: &RedisSession, protocol: RedisProtocol, tag: &str) {
    let mut stream = session
        .scan_collection(
            bytes(b"scan-set"),
            RedisCollectionScanKind::Set,
            RedisCollectionScanOptions::new(PageLimits::new(1, 1, 32, 128), 16, 1, 16, 1_024, 128),
        )
        .unwrap();
    let mut members = BTreeSet::new();
    let mut start = 0_u64;
    while let Some(page) = stream.next_page(identity(), start).await.unwrap() {
        assert!(page.envelope().row_count() <= 1);
        assert_eq!(page.envelope().column_count(), 1);
        for row in 0..page.envelope().row_count() {
            members.insert(page.cell(row, 0).unwrap().bytes().to_vec());
        }
        start += u64::from(page.envelope().row_count());
    }
    assert_eq!(
        members,
        BTreeSet::from([vec![0, 255], b"member".to_vec()]),
        "SSCAN {tag} {protocol:?}"
    );
}

async fn verify_sorted_set_scan(session: &RedisSession, protocol: RedisProtocol, tag: &str) {
    let mut stream = session
        .scan_collection(
            bytes(b"scan-zset"),
            RedisCollectionScanKind::SortedSet,
            RedisCollectionScanOptions::new(PageLimits::new(1, 2, 32, 128), 16, 1, 16, 1_024, 128),
        )
        .unwrap();
    let mut members = BTreeSet::new();
    let mut start = 0_u64;
    while let Some(page) = stream.next_page(identity(), start).await.unwrap() {
        assert!(page.envelope().row_count() <= 1);
        assert_eq!(page.envelope().column_count(), 2);
        for row in 0..page.envelope().row_count() {
            let score_cell = page.cell(row, 1).unwrap();
            assert_eq!(score_cell.kind(), ValueKind::Float64);
            let score = u64::from_be_bytes(score_cell.bytes().try_into().unwrap());
            members.insert((page.cell(row, 0).unwrap().bytes().to_vec(), score));
        }
        start += u64::from(page.envelope().row_count());
    }
    assert_eq!(
        members,
        BTreeSet::from([
            (vec![0, 255], (-1.25_f64).to_bits()),
            (b"member".to_vec(), 2.5_f64.to_bits()),
        ]),
        "ZSCAN {tag} {protocol:?}"
    );
}

async fn verify_empty_collection_scans(session: &RedisSession, protocol: RedisProtocol, tag: &str) {
    for (kind, columns) in [
        (RedisCollectionScanKind::Hash, 2),
        (RedisCollectionScanKind::Set, 1),
        (RedisCollectionScanKind::SortedSet, 2),
    ] {
        let mut stream = session
            .scan_collection(
                bytes(b"missing-collection"),
                kind,
                RedisCollectionScanOptions::new(
                    PageLimits::new(1, columns, 16, 128),
                    8,
                    1,
                    8,
                    128,
                    8,
                ),
            )
            .unwrap();
        let page = stream.next_page(identity(), 0).await.unwrap().unwrap();
        assert_eq!(
            page.envelope().row_count(),
            0,
            "empty {kind:?} {tag} {protocol:?}"
        );
        assert_eq!(page.envelope().column_count(), columns);
        assert_eq!(page.envelope().delivery(), PageDelivery::Final);
        assert!(stream.next_page(identity(), 0).await.unwrap().is_none());
    }
}

async fn verify_pubsub_isolation(
    session: &RedisSession,
    port: u16,
    protocol: RedisProtocol,
    tag: &str,
) {
    let channel = bytes(&[0, 255, 42]);
    assert!(matches!(
        session
            .subscribe(
                channel.clone(),
                RedisSubscriptionOptions::new(PageLimits::new(1, 2, 64, 64), 32, 4_097),
            )
            .await,
        Err(tablerock_engine::RedisError::InvalidLimits)
    ));
    let mut subscription = session
        .subscribe(
            channel.clone(),
            RedisSubscriptionOptions::new(PageLimits::new(2, 2, 256, 128), 128, 4),
        )
        .await
        .unwrap();
    let mut publisher = raw_connection_in_database(port, protocol, 0).await;
    let receivers: usize = redis::cmd("PUBLISH")
        .arg(channel.as_slice())
        .arg(&[1_u8, 0, 255])
        .query_async(&mut publisher)
        .await
        .unwrap();
    assert_eq!(
        receivers, 1,
        "subscription installed for {tag} {protocol:?}"
    );
    let page = tokio::time::timeout(
        Duration::from_secs(5),
        subscription.next_page(identity(), 0),
    )
    .await
    .expect("subscription delivers within five seconds")
    .unwrap()
    .unwrap();
    assert_eq!(page.cell(0, 0).unwrap().bytes(), &[0, 255, 42]);
    assert_eq!(page.cell(0, 1).unwrap().bytes(), &[1, 0, 255]);
    assert!(session.observed_client_id().await.unwrap() > 0);
    assert_eq!(
        session.dispatch_cancel().await.unwrap(),
        tablerock_engine::RedisCancelDispatch::RequestSent
    );
    assert_eq!(
        tokio::time::timeout(
            Duration::from_secs(5),
            subscription.next_page(identity(), 1)
        )
        .await
        .expect("cancel terminates subscription"),
        Err(tablerock_engine::RedisError::ClientCancelled)
    );
    let subscribers: Vec<(Vec<u8>, u64)> = redis::cmd("PUBSUB")
        .arg("NUMSUB")
        .arg(channel.as_slice())
        .query_async(&mut publisher)
        .await
        .unwrap();
    assert_eq!(subscribers, vec![(channel.as_slice().to_vec(), 0)]);

    let overflow_channel = bytes(&[9, 0, 9]);
    let mut overflowing = DriverSession::start_page_stream(
        session,
        DriverPageRequest::RedisSubscribe {
            selector: overflow_channel.clone(),
            kind: RedisSubscriptionKind::Channel,
            options: RedisSubscriptionOptions::new(PageLimits::new(1, 2, 64, 64), 32, 1),
        },
    )
    .await
    .unwrap();
    drop(subscription);
    assert!(matches!(
        session
            .subscribe(
                bytes(b"third-long-operation"),
                RedisSubscriptionOptions::new(PageLimits::new(1, 2, 64, 64), 32, 1),
            )
            .await,
        Err(tablerock_engine::RedisError::SessionBusy)
    ));
    for payload in 0_u8..8 {
        let _: usize = redis::cmd("PUBLISH")
            .arg(overflow_channel.as_slice())
            .arg(&[payload])
            .query_async(&mut publisher)
            .await
            .unwrap();
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(
        overflowing
            .next_page(identity(), 0)
            .await
            .unwrap()
            .is_some()
    );
    let overflow = overflowing.next_page(identity(), 1).await.unwrap_err();
    assert_eq!(overflow.class(), AdapterFailureClass::ResourceLimit);

    let dropped_channel = bytes(&[7, 0, 7]);
    let dropped = session
        .subscribe(
            dropped_channel.clone(),
            RedisSubscriptionOptions::new(PageLimits::new(1, 2, 64, 64), 32, 1),
        )
        .await
        .unwrap();
    drop(dropped);
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let counts: Vec<(Vec<u8>, u64)> = redis::cmd("PUBSUB")
                .arg("NUMSUB")
                .arg(dropped_channel.as_slice())
                .query_async(&mut publisher)
                .await
                .unwrap();
            if counts[0].1 == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("drop unsubscribes within five seconds");
    let replacement = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match session
                .subscribe(
                    dropped_channel.clone(),
                    RedisSubscriptionOptions::new(PageLimits::new(1, 2, 64, 64), 32, 1),
                )
                .await
            {
                Ok(stream) => break stream,
                Err(tablerock_engine::RedisError::SessionBusy) => {
                    tokio::task::yield_now().await;
                }
                Err(error) => panic!("replacement subscription failed: {error:?}"),
            }
        }
    })
    .await
    .expect("drop releases ownership within five seconds");
    drop(replacement);

    let pattern = bytes(&[0, 255, b'*']);
    assert!(matches!(
        session
            .psubscribe(
                pattern.clone(),
                RedisSubscriptionOptions::new(PageLimits::new(1, 2, 128, 64), 64, 2),
            )
            .await,
        Err(tablerock_engine::RedisError::InvalidLimits)
    ));
    assert!(matches!(
        session
            .psubscribe(
                pattern.clone(),
                RedisSubscriptionOptions::new(PageLimits::new(1, 3, 128, 64), 2, 2),
            )
            .await,
        Err(tablerock_engine::RedisError::InvalidLimits)
    ));
    let mut pattern_subscription = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match DriverSession::start_page_stream(
                session,
                DriverPageRequest::RedisSubscribe {
                    selector: pattern.clone(),
                    kind: RedisSubscriptionKind::Pattern,
                    options: RedisSubscriptionOptions::new(PageLimits::new(1, 3, 256, 128), 128, 2),
                },
            )
            .await
            {
                Ok(stream) => break stream,
                Err(error) if error.class() == AdapterFailureClass::InvalidRequest => {
                    tokio::task::yield_now().await;
                }
                Err(error) => panic!("pattern subscription failed: {error:?}"),
            }
        }
    })
    .await
    .expect("pattern subscription starts within five seconds");
    let matching_channel = [0_u8, 255, b'x'];
    let pattern_payload = [255_u8, 0, 1];
    let receivers: usize = redis::cmd("PUBLISH")
        .arg(&matching_channel)
        .arg(&pattern_payload)
        .query_async(&mut publisher)
        .await
        .unwrap();
    assert_eq!(receivers, 1, "pattern receiver for {tag} {protocol:?}");
    let page = tokio::time::timeout(
        Duration::from_secs(5),
        pattern_subscription.next_page(identity(), 0),
    )
    .await
    .expect("pattern subscription delivers within five seconds")
    .unwrap()
    .unwrap();
    assert_eq!(page.envelope().column_count(), 3);
    assert_eq!(page.cell(0, 0).unwrap().bytes(), pattern.as_slice());
    assert_eq!(page.cell(0, 1).unwrap().bytes(), matching_channel);
    assert_eq!(page.cell(0, 2).unwrap().bytes(), pattern_payload);
    let oversized_payload = vec![42_u8; 200];
    let _: usize = redis::cmd("PUBLISH")
        .arg(&matching_channel)
        .arg(&oversized_payload)
        .query_async(&mut publisher)
        .await
        .unwrap();
    let bounded = tokio::time::timeout(
        Duration::from_secs(5),
        pattern_subscription.next_page(identity(), 1),
    )
    .await
    .expect("oversized pattern payload delivers within five seconds")
    .unwrap()
    .unwrap();
    assert_eq!(bounded.cell(0, 2).unwrap().bytes().len(), 128);
    assert_eq!(
        bounded.cell(0, 2).unwrap().truncation(),
        Truncation::Truncated {
            original_byte_len: Some(200)
        }
    );
    assert!(
        bounded
            .envelope()
            .warnings()
            .contains(PageWarning::ByteLimitReached)
    );
    assert_eq!(
        session.dispatch_cancel().await.unwrap(),
        tablerock_engine::RedisCancelDispatch::RequestSent
    );
    let cancelled = tokio::time::timeout(
        Duration::from_secs(5),
        pattern_subscription.next_page(identity(), 2),
    )
    .await
    .expect("pattern cancel terminates subscription")
    .unwrap_err();
    assert_eq!(cancelled.class(), AdapterFailureClass::ClientCancelled);
    let patterns: u64 = redis::cmd("PUBSUB")
        .arg("NUMPAT")
        .query_async(&mut publisher)
        .await
        .unwrap();
    assert_eq!(patterns, 0, "pattern removed for {tag} {protocol:?}");

    let cancel_session = Arc::new(
        RedisSession::connect(
            &RedisConnectConfig::new(text("127.0.0.1"), port, 0, protocol, RedisTlsMode::Disable),
            RedisConnectionSecurity::new(),
        )
        .await
        .unwrap(),
    );
    let _: () = redis::cmd("CLIENT")
        .arg("PAUSE")
        .arg(1_000_u64)
        .arg("ALL")
        .query_async(&mut publisher)
        .await
        .unwrap();
    let setup_session = Arc::clone(&cancel_session);
    let setup_channel = channel.clone();
    let setup = tokio::spawn(async move {
        setup_session
            .subscribe(
                setup_channel,
                RedisSubscriptionOptions::new(PageLimits::new(1, 2, 64, 64), 32, 1),
            )
            .await
            .map(|_| ())
    });
    let started = tokio::time::Instant::now();
    let dispatch = loop {
        let dispatch = cancel_session.dispatch_cancel().await.unwrap();
        if dispatch != tablerock_engine::RedisCancelDispatch::ServerRejected {
            break dispatch;
        }
        tokio::task::yield_now().await;
    };
    assert_eq!(
        dispatch,
        tablerock_engine::RedisCancelDispatch::PreventedBeforeDispatch
    );
    assert_eq!(
        setup.await.unwrap(),
        Err(tablerock_engine::RedisError::ClientCancelled)
    );
    assert!(started.elapsed() < Duration::from_millis(500));
}

async fn verify_pubsub_service_cancellation(port: u16, protocol: RedisProtocol) {
    let session = RedisSession::connect(
        &RedisConnectConfig::new(text("127.0.0.1"), port, 0, protocol, RedisTlsMode::Disable),
        RedisConnectionSecurity::new(),
    )
    .await
    .unwrap();
    let channel = bytes(b"tablerock-service-pubsub");
    let operation_id = support::operation(70);
    let mut service = support::service(1, 2);
    service
        .submit(
            operation_id,
            support::command(71),
            Box::new(session),
            DriverPageRequest::RedisSubscribe {
                selector: channel.clone(),
                kind: RedisSubscriptionKind::Channel,
                options: RedisSubscriptionOptions::new(PageLimits::new(1, 2, 128, 64), 64, 2),
            },
            identity(),
        )
        .await
        .unwrap();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(5), service.next_update(operation_id))
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
        EngineServiceUpdate::Started
    ));
    let cancel = service.cancel(operation_id).unwrap();
    assert_eq!(cancel.core, tablerock_core::CancelRequestOutcome::Requested);
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(5), service.next_update(operation_id))
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
        EngineServiceUpdate::CancelDispatched(CancelDispatch::RequestSent)
            | EngineServiceUpdate::CancelDispatched(CancelDispatch::PreventedBeforeDispatch)
    ));
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(5), service.next_update(operation_id))
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
        EngineServiceUpdate::Terminal(tablerock_core::OperationOutcome::ClientStopped)
    ));
}

async fn verify_ttl_states(session: &RedisSession, protocol: RedisProtocol, tag: &str) {
    assert_eq!(
        session.read_time_to_live(&bytes(&[0, 1])).await.unwrap(),
        tablerock_core::RedisTimeToLive::Missing,
        "missing TTL {tag} {protocol:?}"
    );
    assert_eq!(
        session.read_time_to_live(&bytes(b"plain")).await.unwrap(),
        tablerock_core::RedisTimeToLive::Persistent,
        "persistent TTL {tag} {protocol:?}"
    );

    let expiring = session
        .read_time_to_live(&bytes(b"long-binary-key"))
        .await
        .unwrap();
    assert!(
        matches!(
            expiring,
            tablerock_core::RedisTimeToLive::Expiring {
                remaining_millis: 1..=600_000
            }
        ),
        "finite TTL {tag} {protocol:?}: {expiring:?}"
    );
}

async fn verify_ttl_mutations(
    session: &RedisSession,
    port: u16,
    protocol: RedisProtocol,
    tag: &str,
) {
    let mut fixture = raw_connection_in_database(port, protocol, 0).await;
    let persistent_key = format!("tablerock-ttl-mutation-{tag}-{protocol:?}");
    let _: () = redis::cmd("SET")
        .arg(persistent_key.as_bytes())
        .arg(b"value")
        .query_async(&mut fixture)
        .await
        .unwrap();

    let missing = session
        .apply_reviewed_ttl_mutation(authorized_ttl_plan(
            0,
            b"tablerock-missing-ttl-mutation",
            vec![MutationChange::RedisSetExpiration(
                RedisExpiration::ExpireAfterMillis(10_000),
            )],
        ))
        .await
        .unwrap();
    assert_eq!(missing.application(), RedisTtlApplication::NotApplied);

    let already_persistent = session
        .apply_reviewed_ttl_mutation(authorized_ttl_plan(
            0,
            persistent_key.as_bytes(),
            vec![MutationChange::RedisSetExpiration(RedisExpiration::Persist)],
        ))
        .await
        .unwrap();
    assert_eq!(
        already_persistent.application(),
        RedisTtlApplication::NotApplied
    );

    let expiring = session
        .apply_reviewed_ttl_mutation(authorized_ttl_plan(
            0,
            persistent_key.as_bytes(),
            vec![MutationChange::RedisSetExpiration(
                RedisExpiration::ExpireAfterMillis(60_000),
            )],
        ))
        .await
        .unwrap();
    assert_eq!(expiring.application(), RedisTtlApplication::Applied);
    assert!(matches!(
        session
            .read_time_to_live(&bytes(persistent_key.as_bytes()))
            .await
            .unwrap(),
        tablerock_core::RedisTimeToLive::Expiring {
            remaining_millis: 1..=60_000
        }
    ));
    let persisted = session
        .apply_reviewed_ttl_mutation(authorized_ttl_plan(
            0,
            persistent_key.as_bytes(),
            vec![MutationChange::RedisSetExpiration(RedisExpiration::Persist)],
        ))
        .await
        .unwrap();
    assert_eq!(persisted.application(), RedisTtlApplication::Applied);
    assert_eq!(
        session
            .read_time_to_live(&bytes(persistent_key.as_bytes()))
            .await
            .unwrap(),
        tablerock_core::RedisTimeToLive::Persistent
    );

    let binary_key = [0_u8, 255, 0, b't'];
    let _: () = redis::cmd("SET")
        .arg(&binary_key)
        .arg(&[1_u8, 0, 255])
        .query_async(&mut fixture)
        .await
        .unwrap();
    let binary_expiry = session
        .apply_reviewed_ttl_mutation(authorized_ttl_plan(
            0,
            &binary_key,
            vec![MutationChange::RedisSetExpiration(
                RedisExpiration::ExpireAfterMillis(60_000),
            )],
        ))
        .await
        .unwrap();
    assert_eq!(binary_expiry.application(), RedisTtlApplication::Applied);
    let binary_persist = session
        .apply_reviewed_ttl_mutation(authorized_ttl_plan(
            0,
            &binary_key,
            vec![MutationChange::RedisSetExpiration(RedisExpiration::Persist)],
        ))
        .await
        .unwrap();
    assert_eq!(binary_persist.application(), RedisTtlApplication::Applied);
    let binary_value: Vec<u8> = redis::cmd("GET")
        .arg(&binary_key)
        .query_async(&mut fixture)
        .await
        .unwrap();
    assert_eq!(binary_value, &[1, 0, 255]);

    let _: () = redis::cmd("PEXPIRE")
        .arg(persistent_key.as_bytes())
        .arg(45_000_u64)
        .query_async(&mut fixture)
        .await
        .unwrap();
    let mut database_one = raw_connection_in_database(port, protocol, 1).await;
    let _: () = redis::cmd("SET")
        .arg(persistent_key.as_bytes())
        .arg(b"database-one-sentinel")
        .arg("PX")
        .arg(90_000_u64)
        .query_async(&mut database_one)
        .await
        .unwrap();

    assert_eq!(
        session
            .apply_reviewed_ttl_mutation(authorized_ttl_plan(
                1,
                persistent_key.as_bytes(),
                vec![MutationChange::RedisSetExpiration(RedisExpiration::Persist)],
            ))
            .await,
        Err(tablerock_engine::RedisError::LogicalDatabaseMismatch)
    );
    let database_one_value: Vec<u8> = redis::cmd("GET")
        .arg(persistent_key.as_bytes())
        .query_async(&mut database_one)
        .await
        .unwrap();
    let database_one_ttl: i64 = redis::cmd("PTTL")
        .arg(persistent_key.as_bytes())
        .query_async(&mut database_one)
        .await
        .unwrap();
    assert_eq!(database_one_value, b"database-one-sentinel");
    assert!((1..=90_000).contains(&database_one_ttl));
    assert_eq!(
        session
            .apply_reviewed_ttl_mutation(authorized_ttl_plan(
                0,
                persistent_key.as_bytes(),
                vec![
                    MutationChange::RedisSetExpiration(RedisExpiration::ExpireAfterMillis(1_000),),
                    MutationChange::RedisSetExpiration(RedisExpiration::Persist),
                ],
            ))
            .await,
        Err(tablerock_engine::RedisError::InvalidMutation)
    );
    assert_eq!(
        session
            .apply_reviewed_ttl_mutation(authorized_ttl_plan(
                0,
                persistent_key.as_bytes(),
                vec![MutationChange::RedisSetString {
                    value: bytes(b"replacement"),
                    expiration: RedisExpiration::Preserve,
                }],
            ))
            .await,
        Err(tablerock_engine::RedisError::InvalidMutation)
    );
    let unchanged_value: Vec<u8> = redis::cmd("GET")
        .arg(persistent_key.as_bytes())
        .query_async(&mut fixture)
        .await
        .unwrap();
    let unchanged_ttl: i64 = redis::cmd("PTTL")
        .arg(persistent_key.as_bytes())
        .query_async(&mut fixture)
        .await
        .unwrap();
    assert_eq!(unchanged_value, b"value");
    assert!((1..=45_000).contains(&unchanged_ttl));

    let ambiguous_key = format!("tablerock-ttl-ambiguous-{tag}-{protocol:?}");
    let _: () = redis::cmd("SET")
        .arg(ambiguous_key.as_bytes())
        .arg(b"value")
        .query_async(&mut fixture)
        .await
        .unwrap();
    let policy = RedisRuntimePolicy::new(
        Duration::from_millis(100),
        Duration::from_millis(100),
        1,
        Duration::from_millis(1),
        Duration::from_millis(1),
    )
    .unwrap();
    let ambiguity_session = RedisSession::connect(
        &RedisConnectConfig::new(text("127.0.0.1"), port, 0, protocol, RedisTlsMode::Disable)
            .with_runtime_policy(policy),
        RedisConnectionSecurity::new(),
    )
    .await
    .unwrap();
    let _: () = redis::cmd("CLIENT")
        .arg("PAUSE")
        .arg(300_u64)
        .arg("WRITE")
        .query_async(&mut fixture)
        .await
        .unwrap();
    assert_eq!(
        ambiguity_session
            .apply_reviewed_ttl_mutation(authorized_ttl_plan(
                0,
                ambiguous_key.as_bytes(),
                vec![MutationChange::RedisSetExpiration(
                    RedisExpiration::ExpireAfterMillis(60_000),
                )],
            ))
            .await,
        Err(tablerock_engine::RedisError::WriteOutcomeUnknown)
    );
    tokio::time::sleep(Duration::from_millis(350)).await;
    let remaining: i64 = redis::cmd("PTTL")
        .arg(ambiguous_key.as_bytes())
        .query_async(&mut fixture)
        .await
        .unwrap();
    assert!((1..=60_000).contains(&remaining));
    let deleted: u64 = redis::cmd("DEL")
        .arg(persistent_key.as_bytes())
        .arg(ambiguous_key.as_bytes())
        .arg(&binary_key)
        .query_async(&mut fixture)
        .await
        .unwrap();
    assert_eq!(deleted, 3);
    let database_one_deleted: u64 = redis::cmd("DEL")
        .arg(persistent_key.as_bytes())
        .query_async(&mut database_one)
        .await
        .unwrap();
    assert_eq!(database_one_deleted, 1);
}

#[derive(Debug, Clone, Copy)]
enum PipelineMode {
    Pipelined,
    MultiExec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PipelineOutcome {
    ServerSucceeded,
    ServerRejected,
}

async fn verify_pipeline_partial_failure(port: u16, protocol: RedisProtocol, tag: &str) {
    for mode in [PipelineMode::Pipelined, PipelineMode::MultiExec] {
        let redis = RedisConnectionInfo::default()
            .set_db(0)
            .set_protocol(match protocol {
                RedisProtocol::Resp2 => ProtocolVersion::RESP2,
                RedisProtocol::Resp3 => ProtocolVersion::RESP3,
            });
        let info = ConnectionAddr::Tcp("127.0.0.1".to_owned(), port)
            .into_connection_info()
            .unwrap()
            .set_redis_settings(redis);
        let client = redis::Client::open(info).unwrap();
        let mut connection = client.get_multiplexed_async_connection().await.unwrap();
        let key = format!("tablerock-pipeline-{tag}-{protocol:?}-{mode:?}");
        let mut pipeline = redis::pipe();
        if matches!(mode, PipelineMode::MultiExec) {
            pipeline.atomic();
        }
        pipeline
            .cmd("SET")
            .arg(key.as_bytes())
            .arg(1_u8)
            .cmd("HSET")
            .arg(key.as_bytes())
            .arg(b"field")
            .arg(b"value")
            .cmd("INCR")
            .arg(key.as_bytes())
            .ignore_errors();
        let results: Vec<redis::RedisResult<redis::Value>> =
            pipeline.query_async(&mut connection).await.unwrap();
        let outcomes = results
            .iter()
            .map(|result| {
                if result.is_ok() {
                    PipelineOutcome::ServerSucceeded
                } else {
                    PipelineOutcome::ServerRejected
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(
            outcomes,
            &[
                PipelineOutcome::ServerSucceeded,
                PipelineOutcome::ServerRejected,
                PipelineOutcome::ServerSucceeded,
            ],
            "Redis pipeline outcomes {tag} {protocol:?} {mode:?}"
        );
        let final_counter: u64 = redis::cmd("GET")
            .arg(key.as_bytes())
            .query_async(&mut connection)
            .await
            .unwrap();
        assert_eq!(
            final_counter, 2,
            "Redis does not roll back successful commands {tag} {protocol:?} {mode:?}"
        );
        redis::cmd("DEL")
            .arg(key.as_bytes())
            .exec_async(&mut connection)
            .await
            .unwrap();
    }
}

async fn verify_service_cancellation(
    port: u16,
    protocol: RedisProtocol,
    tag: &str,
    wait_for_server_dispatch: bool,
) {
    let session = RedisSession::connect(
        &RedisConnectConfig::new(text("127.0.0.1"), port, 0, protocol, RedisTlsMode::Disable),
        RedisConnectionSecurity::new(),
    )
    .await
    .unwrap();
    let operation_id = support::operation(60);
    let mut service = support::service(1, 2);
    service
        .submit(
            operation_id,
            support::command(61),
            Box::new(session),
            DriverPageRequest::RedisBlockingPop {
                key: bytes(b"tablerock-cancellation-empty-list"),
                limits: PageLimits::new(1, 2, 256, 128),
                max_cell_bytes: 128,
            },
            identity(),
        )
        .await
        .unwrap();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(5), service.next_update(operation_id))
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
        EngineServiceUpdate::Started
    ));
    if wait_for_server_dispatch {
        wait_until_blocked(port, None).await;
    }
    let cancel = service.cancel(operation_id).unwrap();
    assert_eq!(cancel.core, tablerock_core::CancelRequestOutcome::Requested);
    assert_eq!(
        cancel.runtime,
        Some(tablerock_engine::RuntimeCancelOutcome::Queued)
    );
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(5), service.next_update(operation_id))
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
        EngineServiceUpdate::CancelDispatched(dispatch)
            if dispatch == if wait_for_server_dispatch {
                CancelDispatch::RequestSent
            } else {
                CancelDispatch::PreventedBeforeDispatch
            }
    ));
    assert!(
        matches!(
            tokio::time::timeout(Duration::from_secs(5), service.next_update(operation_id))
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
            EngineServiceUpdate::Terminal(outcome)
                if outcome == if wait_for_server_dispatch {
                    tablerock_core::OperationOutcome::ServerConfirmedCancelled
                } else {
                    tablerock_core::OperationOutcome::ClientStopped
                }
        ),
        "Redis cancellation outcome {tag} {protocol:?}"
    );
}

async fn wait_until_blocked(port: u16, client_id: Option<u64>) {
    let client = redis::Client::open(format!("redis://127.0.0.1:{port}/0")).unwrap();
    let mut inspector = client.get_multiplexed_async_connection().await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let mut command = redis::cmd("CLIENT");
            command.arg("LIST");
            if let Some(client_id) = client_id {
                command.arg("ID").arg(client_id);
            }
            let state: String = command.query_async(&mut inspector).await.unwrap();
            if state.split_whitespace().any(|field| field == "flags=b") {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("Redis reports the operation connection as blocked");
}

async fn verify_blocking_completion(port: u16, protocol: RedisProtocol) {
    let session = RedisSession::connect(
        &RedisConnectConfig::new(text("127.0.0.1"), port, 0, protocol, RedisTlsMode::Disable),
        RedisConnectionSecurity::new(),
    )
    .await
    .unwrap();
    let mut stream = session
        .blocking_pop(
            bytes(b"tablerock-blocking-completion"),
            PageLimits::new(1, 2, 256, 128),
            128,
        )
        .await
        .unwrap();
    let client_id = session.active_blocking_client_id().unwrap();
    assert!(matches!(
        session
            .blocking_pop(
                bytes(b"second-blocking-operation"),
                PageLimits::new(1, 2, 256, 128),
                128,
            )
            .await,
        Err(tablerock_engine::RedisError::SessionBusy)
    ));
    wait_until_blocked(port, Some(client_id)).await;
    let client = redis::Client::open(format!("redis://127.0.0.1:{port}/0")).unwrap();
    let mut producer = client.get_multiplexed_async_connection().await.unwrap();
    let pushed: u64 = redis::cmd("RPUSH")
        .arg(b"tablerock-blocking-completion")
        .arg(&[0_u8, 255])
        .query_async(&mut producer)
        .await
        .unwrap();
    assert_eq!(pushed, 1);
    let page = tokio::time::timeout(Duration::from_secs(5), stream.next_page(identity(), 0))
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(session.active_blocking_client_id(), None);
    assert_eq!(page.envelope().delivery(), PageDelivery::Final);
    assert_eq!(
        page.cell(0, 0).unwrap().bytes(),
        b"tablerock-blocking-completion"
    );
    assert_eq!(page.cell(0, 1).unwrap().bytes(), &[0, 255]);
}

async fn seed(port: u16) {
    for (database, entries) in [
        (
            0,
            vec![
                (vec![0, 255], vec![1, 0, 255, 2]),
                (b"long-binary-key".to_vec(), b"value".to_vec()),
                (b"plain".to_vec(), b"value".to_vec()),
            ],
        ),
        (1, vec![(b"database-one".to_vec(), b"isolated".to_vec())]),
    ] {
        let client = redis::Client::open(format!("redis://127.0.0.1:{port}/{database}")).unwrap();
        let mut connection = None;
        for _ in 0..50 {
            match client.get_multiplexed_async_connection().await {
                Ok(connected) => {
                    connection = Some(connected);
                    break;
                }
                Err(_) => tokio::time::sleep(Duration::from_millis(20)).await,
            }
        }
        let mut connection = connection.expect("Redis fixture accepts connections");
        for (key, value) in entries {
            let _: () = connection.set(key, value).await.unwrap();
        }
        if database == 0 {
            let _: bool = redis::cmd("PEXPIRE")
                .arg(b"long-binary-key")
                .arg(600_000_u64)
                .query_async(&mut connection)
                .await
                .unwrap();
            let _: u64 = redis::cmd("HSET")
                .arg(b"scan-hash")
                .arg(&[0_u8, 255])
                .arg(&[1_u8, 2, 3, 4])
                .arg(b"field")
                .arg(b"value")
                .query_async(&mut connection)
                .await
                .unwrap();
            let _: u64 = redis::cmd("SADD")
                .arg(b"scan-set")
                .arg(&[0_u8, 255])
                .arg(b"member")
                .query_async(&mut connection)
                .await
                .unwrap();
            let _: u64 = redis::cmd("ZADD")
                .arg(b"scan-zset")
                .arg(-1.25_f64)
                .arg(&[0_u8, 255])
                .arg(2.5_f64)
                .arg(b"member")
                .query_async(&mut connection)
                .await
                .unwrap();
        }
    }
}
