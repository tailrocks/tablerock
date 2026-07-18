use std::{sync::Arc, time::Duration};

use rcgen::ExtendedKeyUsagePurpose;
use tablerock_core::{
    BoundedBytes, BoundedText, ByteLimit, CancelDispatch, Engine, PageDelivery, PageIdentity,
    PageLimits, PageWarning, Truncation, ValueKind,
};
use tablerock_engine::{
    AdapterFailureClass, ClickHouseProbeQuery, DriverPageRequest, DriverSession,
    EngineServiceUpdate, PostgresCancellationOutcome, PostgresClientIdentity,
    PostgresConnectConfig, PostgresCopyLimits, PostgresError, PostgresNoticeDelivery,
    PostgresProbeQuery, PostgresSession, PostgresStatementKind, PostgresTlsMaterial,
    PostgresTlsMode,
};

mod support;
mod tls_support;
use testcontainers::{
    CopyDataSource, CopyTargetOptions, GenericImage, ImageExt,
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};
use tls_support::{certificate_authority, leaf_certificate};

struct PostgresTlsFixture {
    ca_pem: String,
    wrong_ca_pem: String,
    server_certificate_pem: String,
    server_private_key_pem: String,
    client_certificate_pem: String,
    client_private_key_pem: String,
}

impl PostgresTlsFixture {
    fn generate() -> Self {
        let ca = certificate_authority("TableRock PostgreSQL test CA");
        let wrong_ca = certificate_authority("Untrusted TableRock test CA");
        let (server_certificate_pem, server_private_key_pem) = leaf_certificate(
            "database.internal",
            ExtendedKeyUsagePurpose::ServerAuth,
            &ca,
        );
        let (client_certificate_pem, client_private_key_pem) =
            leaf_certificate("postgres", ExtendedKeyUsagePurpose::ClientAuth, &ca);
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

fn tls_init_script() -> Vec<u8> {
    br#"#!/bin/sh
set -eu
cp /tablerock-tls/server.crt "$PGDATA/server.crt"
cp /tablerock-tls/server.key "$PGDATA/server.key"
cp /tablerock-tls/ca.crt "$PGDATA/ca.crt"
chmod 600 "$PGDATA/server.key"
chmod 644 "$PGDATA/server.crt" "$PGDATA/ca.crt"
cat >> "$PGDATA/postgresql.conf" <<'EOF'
ssl = on
ssl_cert_file = 'server.crt'
ssl_key_file = 'server.key'
ssl_ca_file = 'ca.crt'
ssl_min_protocol_version = 'TLSv1.2'
EOF
cat > "$PGDATA/pg_hba.conf" <<'EOF'
local all all trust
hostssl all root_only 0.0.0.0/0 trust
hostssl all root_only ::/0 trust
hostssl all postgres 0.0.0.0/0 cert
hostssl all postgres ::/0 cert
EOF
"#
    .to_vec()
}

async fn connect_custom_tls(
    config: &PostgresConnectConfig,
    ca_pem: &[u8],
    identity: Option<(&[u8], &[u8])>,
) -> Result<PostgresSession, PostgresError> {
    let material = match identity {
        Some((certificate, key)) => PostgresTlsMaterial::new(ca_pem)
            .with_client_identity(PostgresClientIdentity::new(certificate, key)),
        None => PostgresTlsMaterial::new(ca_pem),
    };
    PostgresSession::connect_with_tls(config, material).await
}

async fn connect_custom_tls_until_ready(
    config: &PostgresConnectConfig,
    ca_pem: &[u8],
) -> PostgresSession {
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if let Ok(session) = connect_custom_tls(config, ca_pem, None).await {
                break session;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn verifies_custom_roots_client_identity_and_tls_cancellation_on_supported_lines() {
    for tag in ["17.10-alpine", "18.4-alpine"] {
        verify_tls_matrix(tag).await;
    }
}

async fn verify_tls_matrix(tag: &str) {
    let fixture = PostgresTlsFixture::generate();
    let container = GenericImage::new("postgres", tag)
        .with_exposed_port(5432.tcp())
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_HOST_AUTH_METHOD", "trust")
        .with_copy_to(
            CopyTargetOptions::new("/tablerock-tls/server.crt").with_mode(0o644),
            CopyDataSource::Data(fixture.server_certificate_pem.as_bytes().to_vec()),
        )
        .with_copy_to(
            CopyTargetOptions::new("/tablerock-tls/server.key").with_mode(0o644),
            CopyDataSource::Data(fixture.server_private_key_pem.as_bytes().to_vec()),
        )
        .with_copy_to(
            CopyTargetOptions::new("/tablerock-tls/ca.crt").with_mode(0o644),
            CopyDataSource::Data(fixture.ca_pem.as_bytes().to_vec()),
        )
        .with_copy_to(
            CopyTargetOptions::new("/docker-entrypoint-initdb.d/001-tablerock-tls.sh")
                .with_mode(0o755),
            CopyDataSource::Data(tls_init_script()),
        )
        .with_copy_to(
            CopyTargetOptions::new("/docker-entrypoint-initdb.d/002-tablerock-role.sql")
                .with_mode(0o644),
            CopyDataSource::Data(b"CREATE ROLE root_only LOGIN;\n".to_vec()),
        )
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(5432.tcp()).await.unwrap();
    let root_without_override = PostgresConnectConfig::new(
        text("127.0.0.1"),
        port,
        text("postgres"),
        text("root_only"),
        PostgresTlsMode::Required,
    );
    let root_only = root_without_override
        .clone()
        .with_tls_server_name(text("database.internal"));

    let root_session = connect_custom_tls_until_ready(&root_only, fixture.ca_pem.as_bytes()).await;
    let mut page = root_session
        .stream_probe(
            PostgresProbeQuery::BoundedSeries,
            PageLimits::new(4, 8, 256, 256),
            32,
        )
        .await
        .unwrap();
    assert_eq!(
        page.next_page(identity(), 0)
            .await
            .unwrap()
            .unwrap()
            .envelope()
            .row_count(),
        3
    );
    drop(page);
    root_session.shutdown().await.unwrap();

    let plaintext = PostgresConnectConfig::new(
        text("127.0.0.1"),
        port,
        text("postgres"),
        text("root_only"),
        PostgresTlsMode::Disabled,
    );
    assert!(matches!(
        PostgresSession::connect(&plaintext).await,
        Err(PostgresError::Connect)
    ));
    assert!(matches!(
        connect_custom_tls(&root_without_override, fixture.ca_pem.as_bytes(), None).await,
        Err(PostgresError::Connect)
    ));
    assert!(matches!(
        connect_custom_tls(&root_only, fixture.wrong_ca_pem.as_bytes(), None).await,
        Err(PostgresError::Connect)
    ));

    let mutual_tls = PostgresConnectConfig::new(
        text("127.0.0.1"),
        port,
        text("postgres"),
        text("postgres"),
        PostgresTlsMode::Required,
    )
    .with_tls_server_name(text("database.internal"));
    assert!(matches!(
        connect_custom_tls(&mutual_tls, fixture.ca_pem.as_bytes(), None).await,
        Err(PostgresError::Connect)
    ));
    let duplicate_private_keys = format!(
        "{}{}",
        fixture.client_private_key_pem, fixture.client_private_key_pem
    );
    assert!(matches!(
        connect_custom_tls(
            &mutual_tls,
            fixture.ca_pem.as_bytes(),
            Some((
                fixture.client_certificate_pem.as_bytes(),
                duplicate_private_keys.as_bytes(),
            )),
        )
        .await,
        Err(PostgresError::TlsConfiguration)
    ));
    let session = connect_custom_tls(
        &mutual_tls,
        fixture.ca_pem.as_bytes(),
        Some((
            fixture.client_certificate_pem.as_bytes(),
            fixture.client_private_key_pem.as_bytes(),
        )),
    )
    .await
    .unwrap();
    assert_eq!(
        session.cancel_sleep_probe().await.unwrap(),
        PostgresCancellationOutcome::ConfirmedByServer
    );
    for _ in 0..3 {
        assert_eq!(
            session.cancel_completed_probe().await.unwrap(),
            PostgresCancellationOutcome::RequestAcceptedButQueryCompleted
        );
    }
    session.shutdown().await.unwrap();
}

#[tokio::test]
async fn preserves_unknown_mtls_commit_transport_loss_without_downgrade_or_retry() {
    for tag in ["17.10-alpine", "18.4-alpine"] {
        let fixture = PostgresTlsFixture::generate();
        let container = GenericImage::new("postgres", tag)
            .with_exposed_port(5432.tcp())
            .with_wait_for(WaitFor::message_on_stderr(
                "database system is ready to accept connections",
            ))
            .with_env_var("POSTGRES_HOST_AUTH_METHOD", "trust")
            .with_copy_to(
                CopyTargetOptions::new("/tablerock-tls/server.crt").with_mode(0o644),
                CopyDataSource::Data(fixture.server_certificate_pem.as_bytes().to_vec()),
            )
            .with_copy_to(
                CopyTargetOptions::new("/tablerock-tls/server.key").with_mode(0o644),
                CopyDataSource::Data(fixture.server_private_key_pem.as_bytes().to_vec()),
            )
            .with_copy_to(
                CopyTargetOptions::new("/tablerock-tls/ca.crt").with_mode(0o644),
                CopyDataSource::Data(fixture.ca_pem.as_bytes().to_vec()),
            )
            .with_copy_to(
                CopyTargetOptions::new("/docker-entrypoint-initdb.d/001-tablerock-tls.sh")
                    .with_mode(0o755),
                CopyDataSource::Data(tls_init_script()),
            )
            .start()
            .await
            .unwrap();
        let port = container.get_host_port_ipv4(5432.tcp()).await.unwrap();
        let config = PostgresConnectConfig::new(
            text("127.0.0.1"),
            port,
            text("postgres"),
            text("postgres"),
            PostgresTlsMode::Required,
        )
        .with_tls_server_name(text("database.internal"));
        let identity = || {
            Some((
                fixture.client_certificate_pem.as_bytes(),
                fixture.client_private_key_pem.as_bytes(),
            ))
        };
        let session = connect_custom_tls(&config, fixture.ca_pem.as_bytes(), identity())
            .await
            .unwrap();
        let observer = connect_custom_tls(&config, fixture.ca_pem.as_bytes(), identity())
            .await
            .unwrap();
        session
            .prepare_ambiguous_transport_commit_probe()
            .await
            .unwrap();

        let write = session.ambiguous_transport_commit_probe();
        let stop = async {
            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    if observer
                        .ambiguous_transport_commit_waiting_probe()
                        .await
                        .unwrap()
                    {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("mTLS deferred COMMIT reaches its server wait");
            container.stop_with_timeout(Some(1)).await.unwrap();
        };
        let (write, ()) = tokio::join!(write, stop);
        assert_eq!(write, Err(PostgresError::WriteOutcomeUnknown));
        assert_eq!(observer.shutdown().await, Err(PostgresError::Connection));
        assert_eq!(session.shutdown().await, Err(PostgresError::Connection));

        container.start().await.unwrap();
        let recovered_port = container.get_host_port_ipv4(5432.tcp()).await.unwrap();
        let recovery_config = PostgresConnectConfig::new(
            text("127.0.0.1"),
            recovered_port,
            text("postgres"),
            text("postgres"),
            PostgresTlsMode::Required,
        )
        .with_tls_server_name(text("database.internal"));
        let plaintext = PostgresConnectConfig::new(
            text("127.0.0.1"),
            recovered_port,
            text("postgres"),
            text("postgres"),
            PostgresTlsMode::Disabled,
        );
        assert!(matches!(
            PostgresSession::connect(&plaintext).await,
            Err(PostgresError::Connect)
        ));
        let recovered = tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                if let Ok(Ok(session)) = tokio::time::timeout(
                    Duration::from_secs(2),
                    connect_custom_tls(&recovery_config, fixture.ca_pem.as_bytes(), identity()),
                )
                .await
                {
                    break session;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("mTLS PostgreSQL restarts within thirty seconds");
        assert_eq!(
            recovered
                .ambiguous_transport_commit_count_probe()
                .await
                .unwrap(),
            0
        );
        recovered.shutdown().await.unwrap();
    }
}

fn text(value: &str) -> BoundedText {
    BoundedText::copy_from_str(value, ByteLimit::new(128)).unwrap()
}

fn bytes(value: &[u8]) -> BoundedBytes {
    BoundedBytes::copy_from_slice(value, ByteLimit::new(128)).unwrap()
}

#[tokio::test]
async fn distinguishes_server_confirmed_cancellation_from_request_delivery() {
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

    assert_eq!(
        session.cancel_sleep_probe().await.unwrap(),
        PostgresCancellationOutcome::ConfirmedByServer
    );
    assert_eq!(
        session.cancel_completed_probe().await.unwrap(),
        PostgresCancellationOutcome::RequestAcceptedButQueryCompleted
    );
    let mut follow_up = session
        .stream_probe(
            PostgresProbeQuery::BoundedSeries,
            PageLimits::new(4, 8, 256, 256),
            32,
        )
        .await
        .unwrap();
    assert_eq!(
        follow_up
            .next_page(identity(), 0)
            .await
            .unwrap()
            .unwrap()
            .envelope()
            .row_count(),
        3
    );
    drop(follow_up);
    session.shutdown().await.unwrap();
}

#[tokio::test]
async fn classifies_cancel_transport_loss_before_query_disconnect() {
    for tag in ["17.10-alpine", "18.4-alpine"] {
        let container = GenericImage::new("postgres", tag)
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

        let cancellation = session.cancel_transport_loss_probe();
        let stop = async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            container.stop_with_timeout(Some(0)).await.unwrap();
        };
        let (result, ()) = tokio::join!(cancellation, stop);
        assert_eq!(result, Err(PostgresError::CancellationTransport));
        assert_eq!(session.shutdown().await, Err(PostgresError::Connection));
    }
}

fn identity() -> PageIdentity {
    support::identity(Engine::PostgreSql, 1)
}

#[tokio::test]
async fn streams_bounded_pages_from_real_postgres() {
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
    let session = PostgresSession::connect(&config).await.unwrap();
    let driver: &dyn DriverSession = &session;
    assert_eq!(driver.engine(), Engine::PostgreSql);
    let mismatch = match driver
        .start_page_stream(DriverPageRequest::ClickHouseProbe {
            query: ClickHouseProbeQuery::TypedValues,
            query_id: text("must-not-log"),
            limits: PageLimits::new(2, 8, 32, 256),
            max_cell_bytes: 8,
        })
        .await
    {
        Err(error) => error,
        Ok(_) => panic!("PostgreSQL adapter must reject a ClickHouse request"),
    };
    assert_eq!(mismatch.class(), AdapterFailureClass::EngineMismatch);
    let mut stream = driver
        .start_page_stream(DriverPageRequest::PostgreSqlProbe {
            query: PostgresProbeQuery::BoundedSeries,
            limits: PageLimits::new(2, 8, 32, 256),
            max_cell_bytes: 8,
        })
        .await
        .unwrap();

    let first = stream.next_page(identity(), 0).await.unwrap().unwrap();
    assert_eq!(first.envelope().row_count(), 2);
    assert_eq!(first.envelope().delivery(), PageDelivery::Partial);
    assert!(
        first
            .envelope()
            .warnings()
            .contains(PageWarning::RowLimitReached)
    );
    assert!(
        first
            .envelope()
            .warnings()
            .contains(PageWarning::ByteLimitReached)
    );
    assert_eq!(first.cell(0, 0).unwrap().bytes(), b"1");
    assert_eq!(first.cell(1, 0).unwrap().bytes(), b"2");
    assert_eq!(first.cell(0, 1).unwrap().bytes(), "éééé".as_bytes());
    assert!(matches!(
        first.cell(0, 1).unwrap().truncation(),
        Truncation::Truncated {
            original_byte_len: Some(20)
        }
    ));
    assert!(first.cell(0, 2).unwrap().is_null());

    let second = stream.next_page(identity(), 2).await.unwrap().unwrap();
    assert_eq!(second.envelope().row_count(), 1);
    assert_eq!(second.envelope().delivery(), PageDelivery::Final);
    assert_eq!(second.cell(0, 0).unwrap().bytes(), b"3");
    assert!(stream.next_page(identity(), 3).await.unwrap().is_none());
    drop(stream);

    let operation_id = support::operation(10);
    let mut service = support::service(1, 2);
    service
        .submit(
            operation_id,
            support::command(23),
            Arc::new(session),
            DriverPageRequest::PostgreSqlProbe {
                query: PostgresProbeQuery::BoundedSeries,
                limits: PageLimits::new(2, 8, 32, 256),
                max_cell_bytes: 8,
            },
            identity(),
        )
        .await
        .unwrap();
    let mut page_rows = 0_u32;
    loop {
        match service.next_update(operation_id).await.unwrap().unwrap() {
            EngineServiceUpdate::Page(page) => {
                page_rows += page.envelope().row_count();
            }
            EngineServiceUpdate::Terminal(tablerock_core::OperationOutcome::Completed) => break,
            EngineServiceUpdate::Started => {}
            other => panic!("unexpected runtime event: {other:?}"),
        }
    }
    assert_eq!(page_rows, 3);
}

/// Plan 009: 2,500-row browse fixture — 500-row pages, first page before
/// completion, ResultStore admit+pin for scroll FetchPage semantics.
#[tokio::test]
async fn browses_2500_row_table_in_500_row_pages_with_result_store_pin() {
    use tablerock_core::{
        OpenResultOutcome, ResultStore, ResultStoreLimits, StatementText,
    };

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

    session
        .execute_sql(
            "CREATE TABLE browse_2500 (id integer PRIMARY KEY, label text);\
             INSERT INTO browse_2500 SELECT g, 'row-' || g::text FROM generate_series(1, 2500) AS g;",
        )
        .await
        .unwrap();

    let page_limits = PageLimits::new(500, 8, 2 * 1024 * 1024, 64 * 1024);
    let mut stream = session
        .start_page_stream(DriverPageRequest::PostgreSqlStatement {
            statement: StatementText::new("SELECT id, label FROM browse_2500 ORDER BY id").unwrap(),
            parameters: Vec::new(),
            limits: page_limits,
            max_cell_bytes: 64 * 1024,
        })
        .await
        .unwrap();

    let mut store = ResultStore::new(ResultStoreLimits::new(8, 32, 64 * 1024 * 1024).unwrap());
    let id = support::identity(Engine::PostgreSql, 2500);
    assert_eq!(store.open_result(id), Ok(OpenResultOutcome::Opened));

    let mut start_row = 0_u64;
    let mut pages = 0_u32;
    let mut total = 0_u64;
    let mut first_page_seen = false;
    loop {
        match stream.next_page(id, start_row).await.unwrap() {
            Some(page) => {
                let rows = u64::from(page.envelope().row_count());
                assert!(rows <= 500);
                if !first_page_seen {
                    assert_eq!(page.envelope().start_row(), 0);
                    assert_eq!(rows, 500);
                    first_page_seen = true;
                }
                let outcome = store.admit(page).unwrap();
                if start_row == 0 {
                    assert!(store.set_pinned(outcome.admitted(), true));
                }
                start_row = start_row.saturating_add(rows);
                total = total.saturating_add(rows);
                pages += 1;
            }
            None => break,
        }
    }
    assert!(first_page_seen, "first page must render before stream end");
    assert_eq!(total, 2500);
    assert_eq!(pages, 5);
    // Scroll FetchPage: page at row 500 is resident after pump-and-store.
    let key = tablerock_core::PageKey::new(id.result_id(), id.revision(), 500);
    let page2 = store.get(key).expect("second page admitted");
    assert_eq!(page2.envelope().row_count(), 500);
    assert_eq!(page2.cell(0, 0).unwrap().kind(), ValueKind::Signed);
    assert_eq!(
        i64::from_be_bytes(page2.cell(0, 0).unwrap().bytes().try_into().unwrap()),
        501
    );
    assert_eq!(page2.cell(0, 1).unwrap().bytes(), b"row-501");
    // Viewport pin survives further access.
    assert!(store.set_pinned(key, true));
    assert!(stream.next_page(id, start_row).await.unwrap().is_none());
    drop(stream);

    // EngineService path: Started + Page precede Terminal.
    let operation_id = support::operation(2501);
    let mut service = support::service(1, 8);
    service
        .submit(
            operation_id,
            support::command(2501),
            Arc::new(session),
            DriverPageRequest::PostgreSqlStatement {
                statement: StatementText::new("SELECT id FROM browse_2500 ORDER BY id").unwrap(),
                parameters: Vec::new(),
                limits: page_limits,
                max_cell_bytes: 64 * 1024,
            },
            support::identity(Engine::PostgreSql, 2501),
        )
        .await
        .unwrap();
    let mut saw_started = false;
    let mut saw_page = false;
    let mut service_rows = 0_u64;
    loop {
        match service.next_update(operation_id).await.unwrap().unwrap() {
            EngineServiceUpdate::Started => {
                assert!(!saw_page, "Started must precede first Page");
                saw_started = true;
            }
            EngineServiceUpdate::Page(page) => {
                assert!(saw_started, "Page must follow Started");
                saw_page = true;
                service_rows += u64::from(page.envelope().row_count());
            }
            EngineServiceUpdate::Terminal(tablerock_core::OperationOutcome::Completed) => {
                assert!(saw_page, "at least one Page before Terminal");
                break;
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
    assert_eq!(service_rows, 2500);
}

#[tokio::test]
async fn reports_request_delivery_and_server_confirmed_cancellation_through_service() {
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
    let operation_id = support::operation(30);
    let mut service = support::service(1, 2);
    service
        .submit(
            operation_id,
            support::command(31),
            Arc::new(session),
            DriverPageRequest::PostgreSqlProbe {
                query: PostgresProbeQuery::CancellationStream,
                limits: PageLimits::new(1, 2, 128, 128),
                max_cell_bytes: 32,
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
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(5), service.next_update(operation_id))
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
        EngineServiceUpdate::Page(_)
    ));
    let cancel = service.cancel(operation_id).unwrap();
    assert_eq!(cancel.core, tablerock_core::CancelRequestOutcome::Requested);
    assert_eq!(
        cancel.runtime,
        Some(tablerock_engine::RuntimeCancelOutcome::Queued)
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match service.next_update(operation_id).await.unwrap().unwrap() {
                EngineServiceUpdate::Page(_) => {}
                EngineServiceUpdate::CancelDispatched(CancelDispatch::RequestSent) => break,
                other => panic!("unexpected event before cancel dispatch: {other:?}"),
            }
        }
    })
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match service.next_update(operation_id).await.unwrap().unwrap() {
                EngineServiceUpdate::Page(_) => {}
                EngineServiceUpdate::Terminal(
                    tablerock_core::OperationOutcome::ServerConfirmedCancelled,
                ) => break,
                other => panic!("unexpected event before cancel terminal: {other:?}"),
            }
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn streams_typed_values_from_supported_postgres_lines() {
    for tag in ["17.10-alpine", "18.4-alpine"] {
        verify_typed_values(tag).await;
    }
}

#[tokio::test]
async fn bounds_postgresql_notices_and_reports_overflow() {
    for tag in ["17.10-alpine", "18.4-alpine"] {
        let container = GenericImage::new("postgres", tag)
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

        session.emit_notice_probe().await.unwrap();
        let notice = tokio::time::timeout(Duration::from_secs(5), session.next_notice())
            .await
            .unwrap()
            .unwrap();
        let PostgresNoticeDelivery::Notice(notice) = notice else {
            panic!("first PostgreSQL notice cannot overflow");
        };
        assert_eq!(notice.severity(), "NOTICE");
        assert_eq!(notice.code(), "00000");
        assert_eq!(notice.message(), "table-rock-notice");
        assert_eq!(notice.message_truncation(), Truncation::Complete);
        assert_eq!(notice.detail(), Some("table-rock-detail"));
        assert_eq!(notice.detail_truncation(), Some(Truncation::Complete));
        assert_eq!(notice.hint(), Some("table-rock-hint"));
        assert_eq!(notice.hint_truncation(), Some(Truncation::Complete));
        let debug = format!("{notice:?}");
        assert!(!debug.contains("table-rock-notice"));
        assert!(!debug.contains("table-rock-detail"));
        assert!(!debug.contains("table-rock-hint"));

        session.emit_long_notice_probe().await.unwrap();
        let notice = tokio::time::timeout(Duration::from_secs(5), session.next_notice())
            .await
            .unwrap()
            .unwrap();
        let PostgresNoticeDelivery::Notice(notice) = notice else {
            panic!("long PostgreSQL notice cannot overflow");
        };
        assert_eq!(notice.message().len(), 1_024);
        assert!(notice.message().is_char_boundary(notice.message().len()));
        assert_eq!(
            notice.message_truncation(),
            Truncation::Truncated {
                original_byte_len: Some(1_200)
            }
        );
        assert_eq!(notice.detail(), None);
        assert_eq!(notice.hint(), None);

        session.emit_notice_overflow_probe().await.unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(5), session.next_notice())
                .await
                .unwrap(),
            Some(PostgresNoticeDelivery::Overflow { dropped: 6 })
        );
        for expected_index in 1..=64 {
            let delivery = session.next_notice().await.unwrap();
            let PostgresNoticeDelivery::Notice(notice) = delivery else {
                panic!("queued PostgreSQL notice cannot become a second overflow");
            };
            assert_eq!(
                notice.message(),
                format!("table-rock-overflow-{expected_index}")
            );
        }
        session.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn preserves_ordered_postgresql_statement_outcomes() {
    for tag in ["17.10-alpine", "18.4-alpine"] {
        let container = GenericImage::new("postgres", tag)
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
        let outcomes = session.multiple_statement_probe().await.unwrap();
        assert_eq!(outcomes.len(), 4);
        assert_eq!(outcomes[0].ordinal(), 0);
        assert_eq!(outcomes[0].kind(), PostgresStatementKind::Command);
        assert_eq!(outcomes[0].row_count(), 0);
        assert_eq!(outcomes[1].ordinal(), 1);
        assert_eq!(outcomes[1].kind(), PostgresStatementKind::Command);
        assert_eq!(outcomes[1].row_count(), 2);
        assert_eq!(outcomes[2].ordinal(), 2);
        assert_eq!(outcomes[2].kind(), PostgresStatementKind::Command);
        assert_eq!(outcomes[2].row_count(), 1);
        assert_eq!(outcomes[3].ordinal(), 3);
        assert_eq!(outcomes[3].kind(), PostgresStatementKind::Query);
        assert_eq!(outcomes[3].row_count(), 2);
        session.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn streams_bounded_postgresql_copy_in_and_out() {
    for tag in ["17.10-alpine", "18.4-alpine"] {
        let container = GenericImage::new("postgres", tag)
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

        let limits = PostgresCopyLimits::new(2_048, 16_384, 4_096);
        let mut stream = session.copy_out_probe(limits).await.unwrap();
        let mut output = Vec::new();
        let mut expected_ordinal = 0;
        while let Some(chunk) = stream.next_chunk().await.unwrap() {
            assert_eq!(chunk.ordinal(), expected_ordinal);
            assert_eq!(chunk.byte_offset(), output.len() as u64);
            assert!(!format!("{chunk:?}").contains("1\n"));
            output.extend_from_slice(chunk.payload());
            expected_ordinal += 1;
        }
        let expected = (1..=1_000)
            .map(|value| format!("{value}\n"))
            .collect::<String>();
        assert_eq!(output, expected.as_bytes());
        let outcome = stream.outcome().unwrap();
        assert_eq!(outcome.chunk_count(), expected_ordinal);
        assert_eq!(outcome.total_bytes(), expected.len() as u64);
        assert_eq!(outcome.row_count(), None);

        let input = [bytes(b"1\n2"), bytes(b"\n3\n")];
        let outcome = session.copy_in_probe(&input, limits).await.unwrap();
        assert_eq!(outcome.chunk_count(), 2);
        assert_eq!(outcome.total_bytes(), 6);
        assert_eq!(outcome.row_count(), Some(3));

        assert_eq!(
            session
                .copy_in_probe(&input, PostgresCopyLimits::new(1, 128, 256))
                .await,
            Err(PostgresError::CopyLimitExceeded)
        );
        let mut limited = session
            .copy_out_probe(PostgresCopyLimits::new(2_048, 16_384, 1))
            .await
            .unwrap();
        assert_eq!(
            limited.next_chunk().await,
            Err(PostgresError::CopyLimitExceeded)
        );
        drop(limited);

        let malformed = [bytes(b"not-an-integer\n")];
        assert_eq!(
            session.copy_in_probe(&malformed, limits).await,
            Err(PostgresError::Query)
        );
        assert_eq!(
            session
                .copy_in_probe(&input, limits)
                .await
                .unwrap()
                .row_count(),
            Some(3)
        );
        session.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn preserves_unknown_postgresql_write_outcome_without_retry() {
    for tag in ["17.10-alpine", "18.4-alpine"] {
        let container = GenericImage::new("postgres", tag)
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
        let session = PostgresSession::connect(&config).await.unwrap();
        let observer = PostgresSession::connect(&config).await.unwrap();

        assert_eq!(
            session.ambiguous_write_probe().await,
            Err(PostgresError::WriteOutcomeUnknown)
        );
        tokio::time::sleep(Duration::from_millis(400)).await;
        assert_eq!(observer.ambiguous_write_count_probe().await.unwrap(), 1);
        assert_eq!(session.ambiguous_write_count_probe().await.unwrap(), 1);

        observer.shutdown().await.unwrap();
        session.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn preserves_unknown_postgresql_commit_outcome_without_retry() {
    for tag in ["17.10-alpine", "18.4-alpine"] {
        let container = GenericImage::new("postgres", tag)
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
        let session = PostgresSession::connect(&config).await.unwrap();
        let observer = PostgresSession::connect(&config).await.unwrap();

        assert_eq!(
            session.ambiguous_commit_probe().await,
            Err(PostgresError::WriteOutcomeUnknown)
        );
        tokio::time::sleep(Duration::from_millis(1_100)).await;
        assert_eq!(observer.ambiguous_commit_count_probe().await.unwrap(), 1);
        assert_eq!(session.ambiguous_commit_count_probe().await.unwrap(), 1);

        observer.shutdown().await.unwrap();
        session.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn preserves_unknown_postgresql_commit_transport_loss_without_retry() {
    for tag in ["17.10-alpine", "18.4-alpine"] {
        let container = GenericImage::new("postgres", tag)
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
        let session = PostgresSession::connect(&config).await.unwrap();
        let observer = PostgresSession::connect(&config).await.unwrap();
        session
            .prepare_ambiguous_transport_commit_probe()
            .await
            .unwrap();

        let write = session.ambiguous_transport_commit_probe();
        let stop = async {
            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    if observer
                        .ambiguous_transport_commit_waiting_probe()
                        .await
                        .unwrap()
                    {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("deferred COMMIT reaches its server wait");
            container.stop_with_timeout(Some(1)).await.unwrap();
        };
        let (write, ()) = tokio::join!(write, stop);
        assert_eq!(write, Err(PostgresError::WriteOutcomeUnknown));
        assert_eq!(observer.shutdown().await, Err(PostgresError::Connection));
        assert_eq!(session.shutdown().await, Err(PostgresError::Connection));

        container.start().await.unwrap();
        let recovered_port = container.get_host_port_ipv4(5432.tcp()).await.unwrap();
        let recovery_config = PostgresConnectConfig::new(
            text("127.0.0.1"),
            recovered_port,
            text("postgres"),
            text("postgres"),
            PostgresTlsMode::Disabled,
        );
        let recovered = tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                if let Ok(Ok(session)) = tokio::time::timeout(
                    Duration::from_secs(2),
                    PostgresSession::connect(&recovery_config),
                )
                .await
                {
                    break session;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("PostgreSQL restarts within thirty seconds");
        assert_eq!(
            recovered
                .ambiguous_transport_commit_count_probe()
                .await
                .unwrap(),
            0
        );
        recovered.shutdown().await.unwrap();
    }
}

async fn verify_typed_values(tag: &str) {
    let container = GenericImage::new("postgres", tag)
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
    let mut stream = session
        .stream_probe(
            PostgresProbeQuery::TypedValues,
            PageLimits::new(2, 20, 256, 1_024),
            8,
        )
        .await
        .unwrap();
    let page = stream.next_page(identity(), 0).await.unwrap().unwrap();

    assert_eq!(page.envelope().row_count(), 1, "PostgreSQL {tag}");
    assert_eq!(page.envelope().delivery(), PageDelivery::Final);
    assert!(
        !page
            .envelope()
            .warnings()
            .contains(PageWarning::UnknownValues)
    );
    assert!(
        page.envelope()
            .warnings()
            .contains(PageWarning::ByteLimitReached)
    );
    assert_eq!(page.columns()[0].engine_type().name(), "bool");
    assert_eq!(page.columns()[6].engine_type().name(), "numeric");
    assert_eq!(page.cell(0, 0).unwrap().kind(), ValueKind::Boolean);
    assert_eq!(page.cell(0, 0).unwrap().bytes(), &[1]);
    assert_eq!(page.cell(0, 1).unwrap().kind(), ValueKind::Signed);
    assert_eq!(
        i64::from_be_bytes(page.cell(0, 1).unwrap().bytes().try_into().unwrap()),
        -32768
    );
    assert_eq!(
        i64::from_be_bytes(page.cell(0, 2).unwrap().bytes().try_into().unwrap()),
        -2147483648
    );
    assert_eq!(
        i64::from_be_bytes(page.cell(0, 3).unwrap().bytes().try_into().unwrap()),
        -9223372036854775807
    );
    assert_eq!(
        f64::from_bits(u64::from_be_bytes(
            page.cell(0, 4).unwrap().bytes().try_into().unwrap()
        )),
        1.5
    );
    assert_eq!(
        u64::from_be_bytes(page.cell(0, 5).unwrap().bytes().try_into().unwrap()),
        (-0.0_f64).to_bits()
    );
    assert_eq!(page.cell(0, 6).unwrap().kind(), ValueKind::Decimal);
    assert_eq!(page.cell(0, 6).unwrap().bytes(), b"123.450");
    assert_eq!(page.cell(0, 7).unwrap().kind(), ValueKind::Text);
    assert_eq!(page.cell(0, 7).unwrap().bytes(), "éééé".as_bytes());
    assert_eq!(page.cell(0, 8).unwrap().kind(), ValueKind::Binary);
    assert_eq!(page.cell(0, 8).unwrap().bytes(), &[0, 1, 255]);
    assert_eq!(page.cell(0, 9).unwrap().kind(), ValueKind::Text);
    assert_eq!(page.cell(0, 9).unwrap().bytes(), b"123e4567");
    assert_eq!(
        page.cell(0, 9).unwrap().truncation(),
        Truncation::Truncated {
            original_byte_len: Some(36)
        }
    );
    assert_eq!(page.cell(0, 10).unwrap().kind(), ValueKind::Structured);
    assert_eq!(page.cell(0, 10).unwrap().bytes(), b"{\"$array");
    assert!(matches!(
        page.cell(0, 10).unwrap().truncation(),
        Truncation::Truncated {
            original_byte_len: Some(original)
        } if original > 8
    ));
    assert!(page.cell(0, 11).unwrap().is_null());
    for column in [12_u32, 13_u32] {
        assert_eq!(page.cell(0, column).unwrap().kind(), ValueKind::Structured);
        assert_eq!(page.cell(0, column).unwrap().bytes(), b"{\"a\":[1,");
        assert_eq!(
            page.cell(0, column).unwrap().truncation(),
            Truncation::Truncated {
                original_byte_len: Some(14)
            }
        );
    }
    assert_eq!(page.columns()[14].engine_type().name(), "int4range");
    assert_eq!(page.cell(0, 14).unwrap().kind(), ValueKind::Structured);
    assert_eq!(page.cell(0, 14).unwrap().bytes(), b"{\"$range");
    assert!(matches!(
        page.cell(0, 14).unwrap().truncation(),
        Truncation::Truncated {
            original_byte_len: Some(original)
        } if original > 8
    ));
    assert_eq!(page.columns()[15].engine_type().name(), "record");
    assert_eq!(page.cell(0, 15).unwrap().kind(), ValueKind::Structured);
    assert_eq!(page.cell(0, 15).unwrap().bytes(), b"{\"$compo");
    assert!(matches!(
        page.cell(0, 15).unwrap().truncation(),
        Truncation::Truncated {
            original_byte_len: Some(original)
        } if original > 8
    ));
    assert_eq!(page.columns()[16].engine_type().name(), "bytea");
    assert_eq!(page.cell(0, 16).unwrap().kind(), ValueKind::Binary);
    assert_eq!(page.cell(0, 16).unwrap().bytes(), &[0xab; 8]);
    assert_eq!(
        page.cell(0, 16).unwrap().truncation(),
        Truncation::Truncated {
            original_byte_len: Some(16)
        }
    );
    assert!(stream.next_page(identity(), 1).await.unwrap().is_none());
    drop(stream);

    let mut numerics = session
        .stream_probe(
            PostgresProbeQuery::NumericValues,
            PageLimits::new(1, 7, 256, 512),
            64,
        )
        .await
        .unwrap();
    let numeric_page = numerics.next_page(identity(), 0).await.unwrap().unwrap();
    for (column, expected) in [
        (0_u32, "123.450"),
        (1_u32, "-0.0012300"),
        (2_u32, "12345678901234567890.1234567890"),
        (3_u32, "NaN"),
        (4_u32, "Infinity"),
        (5_u32, "-Infinity"),
        (6_u32, "0.000"),
    ] {
        assert_eq!(
            numeric_page.columns()[column as usize].engine_type().name(),
            "numeric"
        );
        assert_eq!(
            numeric_page.cell(0, column).unwrap().kind(),
            ValueKind::Decimal,
            "numeric column {column}: {:?}",
            numeric_page.cell(0, column).unwrap().bytes()
        );
        assert_eq!(
            numeric_page.cell(0, column).unwrap().bytes(),
            expected.as_bytes()
        );
    }
    assert!(numerics.next_page(identity(), 1).await.unwrap().is_none());
    drop(numerics);

    let mut uuids = session
        .stream_probe(
            PostgresProbeQuery::UuidValues,
            PageLimits::new(1, 3, 128, 256),
            36,
        )
        .await
        .unwrap();
    let uuid_page = uuids.next_page(identity(), 0).await.unwrap().unwrap();
    for (column, expected) in [
        (0_u32, "123e4567-e89b-12d3-a456-426614174000"),
        (1_u32, "00000000-0000-0000-0000-000000000000"),
        (2_u32, "ffffffff-ffff-ffff-ffff-ffffffffffff"),
    ] {
        assert_eq!(
            uuid_page.columns()[column as usize].engine_type().name(),
            "uuid"
        );
        assert_eq!(uuid_page.cell(0, column).unwrap().kind(), ValueKind::Text);
        assert_eq!(
            uuid_page.cell(0, column).unwrap().bytes(),
            expected.as_bytes()
        );
        assert_eq!(
            uuid_page.cell(0, column).unwrap().truncation(),
            Truncation::Complete
        );
    }
    assert!(uuids.next_page(identity(), 1).await.unwrap().is_none());
    drop(uuids);

    let mut temporals = session
        .stream_probe(
            PostgresProbeQuery::TemporalValues,
            PageLimits::new(1, 14, 256, 896),
            64,
        )
        .await
        .unwrap();
    let temporal_page = temporals.next_page(identity(), 0).await.unwrap().unwrap();
    for (column, engine_type, expected) in [
        (0_u32, "date", "2000-01-01"),
        (1_u32, "date", "2024-02-29"),
        (2_u32, "time", "24:00:00"),
        (3_u32, "time", "12:34:56.123456"),
        (4_u32, "timestamp", "1999-12-31T23:59:59.999999"),
        (5_u32, "timestamptz", "2024-02-29T05:34:56.123456Z"),
        (6_u32, "date", "infinity"),
        (7_u32, "timestamptz", "-infinity"),
        (8_u32, "timetz", "12:34:56.123456+06:30"),
        (9_u32, "interval", "P14M-3DT-14706.123456S"),
        (10_u32, "date", "0000-01-01"),
        (11_u32, "date", "+10000-12-31"),
        (12_u32, "interval", "infinity"),
        (13_u32, "interval", "-infinity"),
    ] {
        assert_eq!(
            temporal_page.columns()[column as usize]
                .engine_type()
                .name(),
            engine_type
        );
        assert_eq!(
            temporal_page.cell(0, column).unwrap().kind(),
            ValueKind::Temporal,
            "PostgreSQL {tag} temporal column {column} raw {:?}",
            temporal_page.cell(0, column).unwrap().bytes()
        );
        assert_eq!(
            temporal_page.cell(0, column).unwrap().bytes(),
            expected.as_bytes()
        );
    }
    assert!(temporals.next_page(identity(), 1).await.unwrap().is_none());
    drop(temporals);

    let mut arrays = session
        .stream_probe(
            PostgresProbeQuery::ArrayValues,
            PageLimits::new(1, 6, 3_072, 512),
            512,
        )
        .await
        .unwrap();
    let array_page = arrays.next_page(identity(), 0).await.unwrap().unwrap();
    for (column, engine_type, expected) in [
        (
            0_u32,
            "_int4",
            "{\"$array\":{\"dimensions\":[[1,3]],\"values\":[1,null,-2]}}",
        ),
        (
            1_u32,
            "_int4",
            "{\"$array\":{\"dimensions\":[[1,2],[1,2]],\"values\":[[1,2],[3,4]]}}",
        ),
        (
            2_u32,
            "_int4",
            "{\"$array\":{\"dimensions\":[[0,3]],\"values\":[7,8,9]}}",
        ),
        (
            3_u32,
            "_text",
            "{\"$array\":{\"dimensions\":[[1,4]],\"values\":[\"plain\",\"quoted\\\"\",\"NULL\",\"é\"]}}",
        ),
        (
            4_u32,
            "_date",
            "{\"$array\":{\"dimensions\":[[1,2]],\"values\":[\"2024-02-29\",\"2000-01-01\"]}}",
        ),
        (
            5_u32,
            "_int4range",
            "{\"$array\":{\"dimensions\":[[1,2]],\"values\":[{\"$range\":{\"empty\":false,\"lower\":{\"kind\":\"inclusive\",\"value\":1},\"upper\":{\"kind\":\"exclusive\",\"value\":3}}},{\"$range\":{\"empty\":true}}]}}",
        ),
    ] {
        assert_eq!(
            array_page.columns()[column as usize].engine_type().name(),
            engine_type
        );
        assert_eq!(
            array_page.cell(0, column).unwrap().kind(),
            ValueKind::Structured
        );
        assert_eq!(
            array_page.cell(0, column).unwrap().bytes(),
            expected.as_bytes()
        );
        assert_eq!(
            array_page.cell(0, column).unwrap().truncation(),
            Truncation::Complete
        );
    }
    assert!(arrays.next_page(identity(), 1).await.unwrap().is_none());
    drop(arrays);

    let mut ranges = session
        .stream_probe(
            PostgresProbeQuery::RangeValues,
            PageLimits::new(1, 6, 4_096, 512),
            512,
        )
        .await
        .unwrap();
    let range_page = ranges.next_page(identity(), 0).await.unwrap().unwrap();
    for (column, engine_type, expected) in [
        (
            0_u32,
            "int4range",
            "{\"$range\":{\"empty\":false,\"lower\":{\"kind\":\"inclusive\",\"value\":1},\"upper\":{\"kind\":\"exclusive\",\"value\":5}}}",
        ),
        (
            1_u32,
            "int8range",
            "{\"$range\":{\"empty\":false,\"lower\":{\"kind\":\"unbounded\"},\"upper\":{\"kind\":\"exclusive\",\"value\":43}}}",
        ),
        (
            2_u32,
            "numrange",
            "{\"$range\":{\"empty\":false,\"lower\":{\"kind\":\"exclusive\",\"value\":{\"$decimal\":\"1.20\"}},\"upper\":{\"kind\":\"inclusive\",\"value\":{\"$decimal\":\"2.30\"}}}}",
        ),
        (
            3_u32,
            "daterange",
            "{\"$range\":{\"empty\":false,\"lower\":{\"kind\":\"inclusive\",\"value\":\"2024-02-29\"},\"upper\":{\"kind\":\"exclusive\",\"value\":\"2024-03-02\"}}}",
        ),
        (
            4_u32,
            "tstzrange",
            "{\"$range\":{\"empty\":false,\"lower\":{\"kind\":\"inclusive\",\"value\":\"2024-02-29T05:00:00Z\"},\"upper\":{\"kind\":\"exclusive\",\"value\":\"2024-02-29T06:00:00Z\"}}}",
        ),
        (5_u32, "tstzrange", "{\"$range\":{\"empty\":true}}"),
    ] {
        assert_eq!(
            range_page.columns()[column as usize].engine_type().name(),
            engine_type
        );
        assert_eq!(
            range_page.cell(0, column).unwrap().kind(),
            ValueKind::Structured
        );
        assert_eq!(
            range_page.cell(0, column).unwrap().bytes(),
            expected.as_bytes()
        );
        assert_eq!(
            range_page.cell(0, column).unwrap().truncation(),
            Truncation::Complete
        );
    }
    assert!(ranges.next_page(identity(), 1).await.unwrap().is_none());
    drop(ranges);

    let mut multiranges = session
        .stream_probe(
            PostgresProbeQuery::MultirangeValues,
            PageLimits::new(1, 5, 8_192, 512),
            1_024,
        )
        .await
        .unwrap();
    let multirange_page = multiranges.next_page(identity(), 0).await.unwrap().unwrap();
    for (column, engine_type, expected) in [
        (0_u32, "int4multirange", "{\"$multirange\":[]}"),
        (
            1_u32,
            "int4multirange",
            "{\"$multirange\":[{\"$range\":{\"empty\":false,\"lower\":{\"kind\":\"inclusive\",\"value\":1},\"upper\":{\"kind\":\"exclusive\",\"value\":3}}},{\"$range\":{\"empty\":false,\"lower\":{\"kind\":\"inclusive\",\"value\":5},\"upper\":{\"kind\":\"exclusive\",\"value\":8}}}]}",
        ),
        (
            2_u32,
            "int8multirange",
            "{\"$multirange\":[{\"$range\":{\"empty\":false,\"lower\":{\"kind\":\"unbounded\"},\"upper\":{\"kind\":\"exclusive\",\"value\":0}}},{\"$range\":{\"empty\":false,\"lower\":{\"kind\":\"inclusive\",\"value\":10},\"upper\":{\"kind\":\"unbounded\"}}}]}",
        ),
        (
            3_u32,
            "nummultirange",
            "{\"$multirange\":[{\"$range\":{\"empty\":false,\"lower\":{\"kind\":\"exclusive\",\"value\":{\"$decimal\":\"1.20\"}},\"upper\":{\"kind\":\"inclusive\",\"value\":{\"$decimal\":\"2.30\"}}}},{\"$range\":{\"empty\":false,\"lower\":{\"kind\":\"inclusive\",\"value\":{\"$decimal\":\"5.00\"}},\"upper\":{\"kind\":\"exclusive\",\"value\":{\"$decimal\":\"6.00\"}}}}]}",
        ),
        (
            4_u32,
            "datemultirange",
            "{\"$multirange\":[{\"$range\":{\"empty\":false,\"lower\":{\"kind\":\"inclusive\",\"value\":\"2024-02-29\"},\"upper\":{\"kind\":\"exclusive\",\"value\":\"2024-03-02\"}}},{\"$range\":{\"empty\":false,\"lower\":{\"kind\":\"inclusive\",\"value\":\"2024-03-10\"},\"upper\":{\"kind\":\"exclusive\",\"value\":\"2024-03-11\"}}}]}",
        ),
    ] {
        assert_eq!(
            multirange_page.columns()[column as usize]
                .engine_type()
                .name(),
            engine_type
        );
        assert_eq!(
            multirange_page.cell(0, column).unwrap().kind(),
            ValueKind::Structured
        );
        assert_eq!(
            multirange_page.cell(0, column).unwrap().bytes(),
            expected.as_bytes()
        );
        assert_eq!(
            multirange_page.cell(0, column).unwrap().truncation(),
            Truncation::Complete
        );
    }
    assert!(
        multiranges
            .next_page(identity(), 1)
            .await
            .unwrap()
            .is_none()
    );
    drop(multiranges);

    session.prepare_composite_probe().await.unwrap();
    let mut composites = session
        .stream_probe(
            PostgresProbeQuery::CompositeValues,
            PageLimits::new(1, 2, 8_192, 512),
            4_096,
        )
        .await
        .unwrap();
    let composite_page = composites.next_page(identity(), 0).await.unwrap().unwrap();
    for (column, engine_type, expected) in [
        (
            0_u32,
            "tablerock_composite_probe",
            "{\"$composite\":{\"fields\":[{\"name\":\"id\",\"oid\":23,\"type\":\"int4\",\"value\":7},{\"name\":\"label\",\"oid\":25,\"type\":\"text\",\"value\":\"é\"},{\"name\":\"absent\",\"oid\":25,\"type\":\"text\",\"value\":null},{\"name\":\"numbers\",\"oid\":1007,\"type\":\"_int4\",\"value\":{\"$array\":{\"dimensions\":[[1,2]],\"values\":[1,2]}}},{\"name\":\"span\",\"oid\":3912,\"type\":\"daterange\",\"value\":{\"$range\":{\"empty\":false,\"lower\":{\"kind\":\"inclusive\",\"value\":\"2024-02-29\"},\"upper\":{\"kind\":\"exclusive\",\"value\":\"2024-03-02\"}}}}]}}",
        ),
        (
            1_u32,
            "record",
            "{\"$composite\":{\"fields\":[{\"name\":null,\"oid\":23,\"type\":\"int4\",\"value\":7},{\"name\":null,\"oid\":25,\"type\":\"text\",\"value\":\"é\"},{\"name\":null,\"oid\":25,\"type\":\"text\",\"value\":null},{\"name\":null,\"oid\":1007,\"type\":\"_int4\",\"value\":{\"$array\":{\"dimensions\":[[1,2]],\"values\":[1,2]}}}]}}",
        ),
    ] {
        assert_eq!(
            composite_page.columns()[column as usize]
                .engine_type()
                .name(),
            engine_type
        );
        assert_eq!(
            composite_page.cell(0, column).unwrap().kind(),
            ValueKind::Structured
        );
        assert_eq!(
            composite_page.cell(0, column).unwrap().bytes(),
            expected.as_bytes()
        );
        assert_eq!(
            composite_page.cell(0, column).unwrap().truncation(),
            Truncation::Complete
        );
    }
    assert!(composites.next_page(identity(), 1).await.unwrap().is_none());
    drop(composites);

    session.prepare_domain_probe().await.unwrap();
    let mut domains = session
        .stream_probe(
            PostgresProbeQuery::DomainValues,
            PageLimits::new(1, 1, 8_192, 512),
            4_096,
        )
        .await
        .unwrap();
    let domain_page = domains.next_page(identity(), 0).await.unwrap().unwrap();
    assert_eq!(
        domain_page.columns()[0].engine_type().name(),
        "tablerock_domain_container"
    );
    assert_eq!(
        domain_page.cell(0, 0).unwrap().kind(),
        ValueKind::Structured
    );
    let document: serde_json::Value =
        serde_json::from_slice(domain_page.cell(0, 0).unwrap().bytes()).unwrap();
    let fields = document["$composite"]["fields"].as_array().unwrap();
    assert_eq!(fields.len(), 5);
    for (field, name, type_name) in [
        (&fields[0], "positive_domain", "tablerock_positive"),
        (&fields[1], "nested_domain", "tablerock_nested_positive"),
        (&fields[2], "array_domain", "tablerock_ints"),
        (&fields[3], "range_domain", "tablerock_dates"),
        (&fields[4], "composite_domain", "tablerock_composite_domain"),
    ] {
        assert_eq!(field["name"], serde_json::Value::String(name.to_owned()));
        assert_eq!(
            field["type"],
            serde_json::Value::String(type_name.to_owned())
        );
        assert!(field["oid"].as_u64().is_some_and(|oid| oid >= 16_384));
    }
    assert_eq!(fields[0]["value"], serde_json::json!(7));
    assert_eq!(fields[1]["value"], serde_json::json!(8));
    assert_eq!(
        fields[2]["value"],
        serde_json::json!({"$array":{"dimensions":[[1,2]],"values":[1,2]}})
    );
    assert_eq!(
        fields[3]["value"],
        serde_json::json!({"$range":{"empty":false,"lower":{"kind":"inclusive","value":"2024-02-29"},"upper":{"kind":"exclusive","value":"2024-03-02"}}})
    );
    assert_eq!(fields[4]["value"]["$composite"]["fields"][0]["value"], 9);
    assert!(domains.next_page(identity(), 1).await.unwrap().is_none());
    drop(domains);

    session.prepare_enum_probe().await.unwrap();
    let mut enums = session
        .stream_probe(
            PostgresProbeQuery::EnumValues,
            PageLimits::new(1, 2, 256, 512),
            64,
        )
        .await
        .unwrap();
    let enum_page = enums.next_page(identity(), 0).await.unwrap().unwrap();
    for (column, expected) in [(0_u32, "ready"), (1_u32, "café")] {
        assert_eq!(
            enum_page.columns()[column as usize].engine_type().name(),
            "tablerock_status"
        );
        assert_eq!(enum_page.cell(0, column).unwrap().kind(), ValueKind::Text);
        assert_eq!(
            enum_page.cell(0, column).unwrap().bytes(),
            expected.as_bytes()
        );
        assert_eq!(
            enum_page.cell(0, column).unwrap().truncation(),
            Truncation::Complete
        );
    }
    assert!(enums.next_page(identity(), 1).await.unwrap().is_none());
    drop(enums);

    let mut networks = session
        .stream_probe(
            PostgresProbeQuery::NetworkValues,
            PageLimits::new(1, 7, 512, 512),
            64,
        )
        .await
        .unwrap();
    let network_page = networks.next_page(identity(), 0).await.unwrap().unwrap();
    for (column, engine_type, expected) in [
        (0_u32, "inet", "192.0.2.1/24"),
        (1_u32, "inet", "203.0.113.7"),
        (2_u32, "inet", "2001:db8::1/64"),
        (3_u32, "cidr", "192.0.2.0/24"),
        (4_u32, "cidr", "2001:db8::/48"),
        (5_u32, "macaddr", "08:00:2b:01:02:03"),
        (6_u32, "macaddr8", "08:00:2b:01:02:03:04:05"),
    ] {
        assert_eq!(
            network_page.columns()[column as usize].engine_type().name(),
            engine_type
        );
        assert_eq!(
            network_page.cell(0, column).unwrap().kind(),
            ValueKind::Text
        );
        assert_eq!(
            network_page.cell(0, column).unwrap().bytes(),
            expected.as_bytes()
        );
        assert_eq!(
            network_page.cell(0, column).unwrap().truncation(),
            Truncation::Complete
        );
    }
    assert!(networks.next_page(identity(), 1).await.unwrap().is_none());
    drop(networks);

    let mut bit_strings = session
        .stream_probe(
            PostgresProbeQuery::BitValues,
            PageLimits::new(1, 4, 256, 512),
            64,
        )
        .await
        .unwrap();
    let bit_page = bit_strings.next_page(identity(), 0).await.unwrap().unwrap();
    for (column, engine_type, expected) in [
        (0_u32, "bit", "10100101"),
        (1_u32, "varbit", "10101"),
        (2_u32, "varbit", ""),
        (3_u32, "varbit", "111100001010"),
    ] {
        assert_eq!(
            bit_page.columns()[column as usize].engine_type().name(),
            engine_type
        );
        assert_eq!(bit_page.cell(0, column).unwrap().kind(), ValueKind::Text);
        assert_eq!(
            bit_page.cell(0, column).unwrap().bytes(),
            expected.as_bytes()
        );
        assert_eq!(
            bit_page.cell(0, column).unwrap().truncation(),
            Truncation::Complete
        );
    }
    assert!(
        bit_strings
            .next_page(identity(), 1)
            .await
            .unwrap()
            .is_none()
    );
    drop(bit_strings);

    let mut identifiers = session
        .stream_probe(
            PostgresProbeQuery::IdentifierValues,
            PageLimits::new(1, 15, 512, 512),
            64,
        )
        .await
        .unwrap();
    let identifier_page = identifiers.next_page(identity(), 0).await.unwrap().unwrap();
    for (column, engine_type, expected) in [
        (0_u32, "oid", u64::from(u32::MAX)),
        (1_u32, "xid", u64::from(u32::MAX)),
        (3_u32, "xid8", u64::MAX),
        (4_u32, "regclass", 1_259),
        (5_u32, "regtype", 23),
        (6_u32, "regnamespace", 11),
        (7_u32, "regrole", 10),
        (8_u32, "regconfig", 3_748),
        (9_u32, "regdictionary", 3_765),
        (10_u32, "regcollation", 950),
        (11_u32, "regproc", 1_299),
        (12_u32, "regprocedure", 1_299),
        (13_u32, "regoper", 96),
        (14_u32, "regoperator", 96),
    ] {
        assert_eq!(
            identifier_page.columns()[column as usize]
                .engine_type()
                .name(),
            engine_type
        );
        assert_eq!(
            identifier_page.cell(0, column).unwrap().kind(),
            ValueKind::Unsigned
        );
        assert_eq!(
            u64::from_be_bytes(
                identifier_page
                    .cell(0, column)
                    .unwrap()
                    .bytes()
                    .try_into()
                    .unwrap()
            ),
            expected
        );
    }
    assert_eq!(identifier_page.columns()[2].engine_type().name(), "cid");
    assert_eq!(
        identifier_page.cell(0, 2).unwrap().kind(),
        ValueKind::Unsigned
    );
    assert!(
        identifiers
            .next_page(identity(), 1)
            .await
            .unwrap()
            .is_none()
    );
    drop(identifiers);

    let mut lsns = session
        .stream_probe(
            PostgresProbeQuery::LsnValues,
            PageLimits::new(1, 3, 256, 512),
            64,
        )
        .await
        .unwrap();
    let lsn_page = lsns.next_page(identity(), 0).await.unwrap().unwrap();
    for (column, expected) in [
        (0_u32, "0/0"),
        (1_u32, "16/B374D848"),
        (2_u32, "FFFFFFFF/FFFFFFFF"),
    ] {
        assert_eq!(
            lsn_page.columns()[column as usize].engine_type().name(),
            "pg_lsn"
        );
        assert_eq!(lsn_page.cell(0, column).unwrap().kind(), ValueKind::Text);
        assert_eq!(
            lsn_page.cell(0, column).unwrap().bytes(),
            expected.as_bytes()
        );
        assert_eq!(
            lsn_page.cell(0, column).unwrap().truncation(),
            Truncation::Complete
        );
    }
    assert!(lsns.next_page(identity(), 1).await.unwrap().is_none());
    drop(lsns);

    let mut tids = session
        .stream_probe(
            PostgresProbeQuery::TidValues,
            PageLimits::new(1, 3, 512, 512),
            128,
        )
        .await
        .unwrap();
    let tid_page = tids.next_page(identity(), 0).await.unwrap().unwrap();
    for (column, expected) in [
        (0_u32, "{\"$tid\":{\"block\":0,\"offset\":1}}"),
        (1_u32, "{\"$tid\":{\"block\":4294967295,\"offset\":65535}}"),
    ] {
        assert_eq!(
            tid_page.columns()[column as usize].engine_type().name(),
            "tid"
        );
        assert_eq!(
            tid_page.cell(0, column).unwrap().kind(),
            ValueKind::Structured
        );
        assert_eq!(
            tid_page.cell(0, column).unwrap().bytes(),
            expected.as_bytes()
        );
    }
    assert_eq!(tid_page.columns()[2].engine_type().name(), "tid");
    assert_eq!(tid_page.cell(0, 2).unwrap().kind(), ValueKind::Structured);
    let live_tid: serde_json::Value =
        serde_json::from_slice(tid_page.cell(0, 2).unwrap().bytes()).unwrap();
    assert!(live_tid["$tid"]["block"].is_u64());
    assert!(live_tid["$tid"]["offset"].is_u64());
    assert!(tids.next_page(identity(), 1).await.unwrap().is_none());
    drop(tids);

    let mut oid_vectors = session
        .stream_probe(
            PostgresProbeQuery::OidVectorValues,
            PageLimits::new(1, 3, 512, 512),
            128,
        )
        .await
        .unwrap();
    let oid_vector_page = oid_vectors.next_page(identity(), 0).await.unwrap().unwrap();
    for (column, expected) in [
        (0_u32, "{\"$oidvector\":[23,25,1043]}"),
        (1_u32, "{\"$oidvector\":[]}"),
        (2_u32, "{\"$oidvector\":[4294967295,0]}"),
    ] {
        assert_eq!(
            oid_vector_page.columns()[column as usize]
                .engine_type()
                .name(),
            "oidvector"
        );
        assert_eq!(
            oid_vector_page.cell(0, column).unwrap().kind(),
            ValueKind::Structured
        );
        assert_eq!(
            oid_vector_page.cell(0, column).unwrap().bytes(),
            expected.as_bytes()
        );
        assert_eq!(
            oid_vector_page.cell(0, column).unwrap().truncation(),
            Truncation::Complete
        );
    }
    assert!(
        oid_vectors
            .next_page(identity(), 1)
            .await
            .unwrap()
            .is_none()
    );
    drop(oid_vectors);

    let mut snapshots = session
        .stream_probe(
            PostgresProbeQuery::SnapshotValues,
            PageLimits::new(1, 3, 512, 512),
            128,
        )
        .await
        .unwrap();
    let snapshot_page = snapshots.next_page(identity(), 0).await.unwrap().unwrap();
    for (column, type_name, expected) in [
        (
            0_u32,
            "pg_snapshot",
            "{\"$snapshot\":{\"xmin\":10,\"xmax\":20,\"in_progress\":[10,14,15]}}",
        ),
        (
            1_u32,
            "txid_snapshot",
            "{\"$snapshot\":{\"xmin\":10,\"xmax\":20,\"in_progress\":[10,14,15]}}",
        ),
        (
            2_u32,
            "pg_snapshot",
            "{\"$snapshot\":{\"xmin\":10,\"xmax\":20,\"in_progress\":[]}}",
        ),
    ] {
        assert_eq!(
            snapshot_page.columns()[column as usize]
                .engine_type()
                .name(),
            type_name
        );
        assert_eq!(
            snapshot_page.cell(0, column).unwrap().kind(),
            ValueKind::Structured
        );
        assert_eq!(
            snapshot_page.cell(0, column).unwrap().bytes(),
            expected.as_bytes()
        );
        assert_eq!(
            snapshot_page.cell(0, column).unwrap().truncation(),
            Truncation::Complete
        );
    }
    assert!(snapshots.next_page(identity(), 1).await.unwrap().is_none());
    drop(snapshots);

    let mut parameters = session
        .stream_probe(
            PostgresProbeQuery::Parameters,
            PageLimits::new(2, 6, 256, 512),
            64,
        )
        .await
        .unwrap();
    let page = parameters.next_page(identity(), 0).await.unwrap().unwrap();
    assert_eq!(page.envelope().row_count(), 1, "PostgreSQL {tag}");
    assert_eq!(page.envelope().column_count(), 6);
    assert_eq!(page.envelope().delivery(), PageDelivery::Final);
    assert_eq!(page.cell(0, 0).unwrap().kind(), ValueKind::Text);
    assert_eq!(page.cell(0, 0).unwrap().bytes(), "parameter-é".as_bytes());
    assert_eq!(page.cell(0, 1).unwrap().kind(), ValueKind::Signed);
    assert_eq!(
        i64::from_be_bytes(page.cell(0, 1).unwrap().bytes().try_into().unwrap()),
        -9_223_372_036_854_775_000_i64
    );
    assert_eq!(page.cell(0, 2).unwrap().kind(), ValueKind::Binary);
    assert_eq!(page.cell(0, 2).unwrap().bytes(), &[0, 1, 255, 0]);
    assert_eq!(page.cell(0, 3).unwrap().kind(), ValueKind::Boolean);
    assert_eq!(page.cell(0, 3).unwrap().bytes(), &[0]);
    assert!(page.cell(0, 4).unwrap().is_null());
    assert_eq!(page.cell(0, 5).unwrap().kind(), ValueKind::Structured);
    assert_eq!(page.columns()[5].engine_type().name(), "_int4");
    assert_eq!(
        page.cell(0, 5).unwrap().bytes(),
        b"{\"$array\":{\"dimensions\":[[1,3]],\"values\":[1,-2,3]}}"
    );
    assert!(parameters.next_page(identity(), 1).await.unwrap().is_none());
    drop(parameters);
    session.shutdown().await.unwrap();
}

#[tokio::test]
async fn persistent_session_runs_statement_cancel_health_and_reuses_connection() {
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
    let session_id =
        tablerock_core::SessionId::from_parts(tablerock_core::IdParts::new(0, 900).unwrap())
            .unwrap();
    let mut service = support::service(4, 4);
    let handle = service
        .register_session(session_id, Box::new(session))
        .unwrap();

    // Statement 1: typed rows.
    let op1 = support::operation(901);
    service
        .submit(
            op1,
            support::command(901),
            Arc::clone(&handle),
            DriverPageRequest::PostgreSqlStatement {
                statement: tablerock_core::StatementText::new(
                    "SELECT 1::int4 AS n UNION ALL SELECT 2::int4 ORDER BY 1",
                )
                .unwrap(),
                parameters: Vec::new(),
                limits: PageLimits::new(10, 8, 4096, 256),
                max_cell_bytes: 64,
            },
            support::identity(Engine::PostgreSql, 901),
        )
        .await
        .unwrap();
    let mut rows = 0_u32;
    loop {
        match service.next_update(op1).await.unwrap().unwrap() {
            EngineServiceUpdate::Started => {}
            EngineServiceUpdate::Page(page) => rows += page.envelope().row_count(),
            EngineServiceUpdate::Terminal(tablerock_core::OperationOutcome::Completed) => break,
            other => panic!("unexpected first statement event: {other:?}"),
        }
    }
    assert_eq!(rows, 2);

    // Statement 2: reuses the same registered session.
    let op2 = support::operation(902);
    service
        .submit(
            op2,
            support::command(902),
            Arc::clone(&handle),
            DriverPageRequest::PostgreSqlStatement {
                statement: tablerock_core::StatementText::new("SELECT 'reuse'::text AS label")
                    .unwrap(),
                parameters: Vec::new(),
                limits: PageLimits::new(10, 8, 4096, 256),
                max_cell_bytes: 64,
            },
            support::identity(Engine::PostgreSql, 902),
        )
        .await
        .unwrap();
    loop {
        match service.next_update(op2).await.unwrap().unwrap() {
            EngineServiceUpdate::Started | EngineServiceUpdate::Page(_) => {}
            EngineServiceUpdate::Terminal(tablerock_core::OperationOutcome::Completed) => break,
            other => panic!("unexpected second statement event: {other:?}"),
        }
    }

    // Cancel a long statement expressed as caller SQL.
    let op3 = support::operation(903);
    service
        .submit(
            op3,
            support::command(903),
            Arc::clone(&handle),
            DriverPageRequest::PostgreSqlStatement {
                statement: tablerock_core::StatementText::new(
                    "SELECT g FROM generate_series(1, 1000000) AS g, pg_sleep(0.05)",
                )
                .unwrap(),
                parameters: Vec::new(),
                limits: PageLimits::new(1, 2, 128, 128),
                max_cell_bytes: 32,
            },
            support::identity(Engine::PostgreSql, 903),
        )
        .await
        .unwrap();
    assert!(matches!(
        service.next_update(op3).await.unwrap().unwrap(),
        EngineServiceUpdate::Started
    ));
    let _ = service.next_update(op3).await.unwrap(); // may be page or still starting
    service.cancel(op3).unwrap();
    loop {
        match service.next_update(op3).await.unwrap().unwrap() {
            EngineServiceUpdate::Page(_)
            | EngineServiceUpdate::CancelDispatched(_)
            | EngineServiceUpdate::Started => {}
            EngineServiceUpdate::Terminal(outcome) => {
                assert!(
                    matches!(
                        outcome,
                        tablerock_core::OperationOutcome::ServerConfirmedCancelled
                            | tablerock_core::OperationOutcome::CompletedBeforeCancel
                            | tablerock_core::OperationOutcome::Completed
                    ),
                    "cancel terminal was {outcome:?}"
                );
                break;
            }
        }
    }

    // Health after cancel.
    let health = handle.health().await.unwrap();
    assert!(health.server_reachable());
    assert_eq!(health.engine(), Engine::PostgreSql);

    // Syntax error surfaces as query failure; session remains usable.
    let op4 = support::operation(904);
    service
        .submit(
            op4,
            support::command(904),
            Arc::clone(&handle),
            DriverPageRequest::PostgreSqlStatement {
                statement: tablerock_core::StatementText::new("SELEC this_is_not_valid").unwrap(),
                parameters: Vec::new(),
                limits: PageLimits::new(10, 8, 4096, 256),
                max_cell_bytes: 64,
            },
            support::identity(Engine::PostgreSql, 904),
        )
        .await
        .unwrap();
    loop {
        match service.next_update(op4).await.unwrap().unwrap() {
            EngineServiceUpdate::Started => {}
            EngineServiceUpdate::Terminal(tablerock_core::OperationOutcome::Failed) => break,
            other => panic!("expected failed syntax event, got {other:?}"),
        }
    }
    let health = handle.health().await.unwrap();
    assert!(health.server_reachable());

    // Empty statement rejected pre-I/O path (InvalidLimits -> Query/Invalid).
    let empty = handle
        .start_page_stream(DriverPageRequest::PostgreSqlStatement {
            statement: tablerock_core::StatementText::new("").unwrap(),
            parameters: Vec::new(),
            limits: PageLimits::new(10, 8, 4096, 256),
            max_cell_bytes: 64,
        })
        .await;
    assert!(empty.is_err());

    drop(handle);
    service.disconnect(session_id).await.unwrap();
}

#[tokio::test]
async fn lists_catalog_databases_schemas_and_relations_with_hostile_names() {
    use tablerock_core::{
        BoundedText, ByteLimit, CatalogChildrenState, CatalogNodeKind, PageLimits,
        PostgreSqlObjectKind,
    };
    use tablerock_engine::{CatalogExactness, CatalogRequest, DriverSession};

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

    session
        .execute_sql(
            r#"
            DROP SCHEMA IF EXISTS "カタログ" CASCADE;
            CREATE SCHEMA "カタログ";
            CREATE TABLE "カタログ"."users" (id int);
            CREATE VIEW "カタログ"."v_users" AS SELECT 1 AS id;
            CREATE TABLE "カタログ"."semi;--x" (id int);
            CREATE FUNCTION "カタログ".add_one(x int) RETURNS int LANGUAGE sql AS $$ SELECT x + 1 $$;
            "#,
        )
        .await
        .unwrap();

    let limits = PageLimits::new(500, 8, 64 * 1024, 256);
    let databases = session
        .catalog(CatalogRequest::PostgreSqlDatabases { limits })
        .await
        .unwrap();
    assert!(
        databases
            .nodes()
            .iter()
            .any(|n| n.name() == "postgres" && n.kind() == CatalogNodeKind::PostgreSqlDatabase)
    );

    let schemas = session
        .catalog(CatalogRequest::PostgreSqlSchemas {
            database: BoundedText::copy_from_str("postgres", ByteLimit::new(64)).unwrap(),
            limits,
        })
        .await
        .unwrap();
    assert!(schemas.nodes().iter().any(|n| n.name() == "カタログ"));
    assert!(schemas.nodes().iter().any(|n| n.name() == "public"));

    let relations = session
        .catalog(CatalogRequest::PostgreSqlRelations {
            database: BoundedText::copy_from_str("postgres", ByteLimit::new(64)).unwrap(),
            schema: BoundedText::copy_from_str("カタログ", ByteLimit::new(64)).unwrap(),
            limits,
        })
        .await
        .unwrap();
    let names: Vec<_> = relations
        .nodes()
        .iter()
        .map(|n| n.name().to_owned())
        .collect();
    assert!(names.contains(&"users".to_owned()));
    assert!(names.contains(&"v_users".to_owned()));
    assert!(
        names.contains(&"semi;--x".to_owned()),
        "hostile name listed: {names:?}"
    );
    assert!(names.contains(&"add_one".to_owned()));
    let function = relations
        .nodes()
        .iter()
        .find(|n| n.name() == "add_one")
        .unwrap();
    assert_eq!(
        function.kind(),
        CatalogNodeKind::PostgreSqlObject(PostgreSqlObjectKind::Function)
    );
    assert_eq!(function.children(), CatalogChildrenState::NotApplicable);
    assert!(function.engine_type().is_some());
    assert_eq!(relations.exactness(), CatalogExactness::Exact);
}

#[tokio::test]
async fn applies_authorized_update_in_transaction_and_conflicts_on_zero_rows() {
    use tablerock_core::{
        BoundedText, ByteLimit, ContextId, FieldValue, IdParts, MutationChange, MutationId,
        MutationPlan, MutationPlanLimits, MutationTarget, OperationScope, OwnedValue, ProfileId,
        ReviewTokenId, Revision, SessionId, Truncation,
    };
    use tablerock_engine::{MutationTransactionState, PostgresSession};

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

    session
        .execute_sql(
            "CREATE TABLE mut_users (id int PRIMARY KEY, name text);
             INSERT INTO mut_users VALUES (1, 'alice');",
        )
        .await
        .unwrap();

    fn bt(s: &str) -> BoundedText {
        BoundedText::copy_from_str(s, ByteLimit::new(10_000)).unwrap()
    }
    fn field(name: &str, value: OwnedValue) -> FieldValue {
        FieldValue::new(bt(name), value)
    }
    let limits = MutationPlanLimits::new(8, 16, 4096, 4096, 60_000).unwrap();
    let scope = OperationScope::new(
        ProfileId::from_parts(IdParts::new(1, 1).unwrap()).unwrap(),
        SessionId::from_parts(IdParts::new(1, 2).unwrap()).unwrap(),
        ContextId::from_parts(IdParts::new(1, 3).unwrap()).unwrap(),
    );
    let target = MutationTarget::PostgreSqlRelation {
        database: bt("postgres"),
        schema: bt("public"),
        relation: bt("mut_users"),
    };

    // Happy path: update one row.
    let plan = MutationPlan::new(
        MutationId::from_parts(IdParts::new(1, 10).unwrap()).unwrap(),
        scope,
        Revision::INITIAL,
        target.clone(),
        vec![MutationChange::UpdateRow {
            locator: vec![field("id", OwnedValue::signed(1))],
            assignments: vec![field(
                "name",
                OwnedValue::text(bt("bob"), Truncation::Complete).unwrap(),
            )],
        }],
        limits,
    )
    .unwrap();
    // Review window must be within MutationPlanLimits.max_review_validity_ms.
    let reviewed = plan
        .review(
            ReviewTokenId::from_parts(IdParts::new(1, 11).unwrap()).unwrap(),
            1_000,
            30_000,
        )
        .unwrap();
    let authorized = reviewed
        .authorize(5_000, scope, Revision::INITIAL)
        .unwrap();
    let outcome = session.apply_authorized_mutation(authorized).await.unwrap();
    assert_eq!(outcome.transaction, MutationTransactionState::Committed);
    assert!(matches!(
        &outcome.changes[0],
        tablerock_engine::MutationChangeOutcome::Applied {
            rows_affected: 1,
            ..
        }
    ));
    assert_eq!(outcome.changes.len(), 1);

    // Conflict: update non-existent id → 0 rows → rollback report.
    let plan2 = MutationPlan::new(
        MutationId::from_parts(IdParts::new(1, 12).unwrap()).unwrap(),
        scope,
        Revision::INITIAL,
        target,
        vec![MutationChange::UpdateRow {
            locator: vec![field("id", OwnedValue::signed(999))],
            assignments: vec![field(
                "name",
                OwnedValue::text(bt("ghost"), Truncation::Complete).unwrap(),
            )],
        }],
        limits,
    )
    .unwrap();
    let reviewed2 = plan2
        .review(
            ReviewTokenId::from_parts(IdParts::new(1, 13).unwrap()).unwrap(),
            1_000,
            30_000,
        )
        .unwrap();
    let authorized2 = reviewed2
        .authorize(5_000, scope, Revision::INITIAL)
        .unwrap();
    let conflict = session.apply_authorized_mutation(authorized2).await.unwrap();
    assert_eq!(conflict.transaction, MutationTransactionState::RolledBack);
    assert!(matches!(
        &conflict.changes[0],
        tablerock_engine::MutationChangeOutcome::Conflict { rows_affected: 0, .. }
    ));

    // Insert + delete happy path in one authorized multi-change plan.
    let multi = MutationPlan::new(
        MutationId::from_parts(IdParts::new(1, 14).unwrap()).unwrap(),
        scope,
        Revision::INITIAL,
        MutationTarget::PostgreSqlRelation {
            database: bt("postgres"),
            schema: bt("public"),
            relation: bt("mut_users"),
        },
        vec![
            MutationChange::InsertRow {
                values: vec![
                    field("id", OwnedValue::signed(2)),
                    field(
                        "name",
                        OwnedValue::text(bt("carol"), Truncation::Complete).unwrap(),
                    ),
                ],
            },
            MutationChange::DeleteRow {
                locator: vec![field("id", OwnedValue::signed(2))],
            },
        ],
        limits,
    )
    .unwrap();
    let multi_auth = multi
        .review(
            ReviewTokenId::from_parts(IdParts::new(1, 15).unwrap()).unwrap(),
            1_000,
            30_000,
        )
        .unwrap()
        .authorize(5_000, scope, Revision::INITIAL)
        .unwrap();
    let multi_out = session
        .apply_authorized_mutation(multi_auth)
        .await
        .unwrap();
    assert_eq!(multi_out.transaction, MutationTransactionState::Committed);
    assert_eq!(multi_out.changes.len(), 2);

    // Constraint violation → Failed + rollback; session still usable.
    let bad = MutationPlan::new(
        MutationId::from_parts(IdParts::new(1, 16).unwrap()).unwrap(),
        scope,
        Revision::INITIAL,
        MutationTarget::PostgreSqlRelation {
            database: bt("postgres"),
            schema: bt("public"),
            relation: bt("mut_users"),
        },
        vec![MutationChange::InsertRow {
            values: vec![
                field("id", OwnedValue::signed(1)), // duplicate PK
                field(
                    "name",
                    OwnedValue::text(bt("dup"), Truncation::Complete).unwrap(),
                ),
            ],
        }],
        limits,
    )
    .unwrap();
    let bad_auth = bad
        .review(
            ReviewTokenId::from_parts(IdParts::new(1, 17).unwrap()).unwrap(),
            1_000,
            30_000,
        )
        .unwrap()
        .authorize(5_000, scope, Revision::INITIAL)
        .unwrap();
    let bad_out = session.apply_authorized_mutation(bad_auth).await.unwrap();
    assert_eq!(bad_out.transaction, MutationTransactionState::RolledBack);
    assert!(matches!(
        &bad_out.changes[0],
        tablerock_engine::MutationChangeOutcome::Failed { .. }
    ));

    // Primary-key column proof for editability.
    let pk = session
        .relation_primary_key_columns("public", "mut_users")
        .await
        .unwrap();
    assert_eq!(pk, vec!["id".to_owned()]);
    let no_pk = session
        .relation_primary_key_columns("public", "does_not_exist")
        .await
        .unwrap();
    assert!(no_pk.is_empty());

    // Session still usable.
    let health = session.health_check().await;
    assert!(health.is_ok());
}
