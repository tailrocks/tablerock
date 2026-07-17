use std::time::Duration;

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
            Box::new(session),
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
            Box::new(session),
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
            PageLimits::new(2, 16, 256, 512),
            8,
        )
        .await
        .unwrap();
    let page = stream.next_page(identity(), 0).await.unwrap().unwrap();

    assert_eq!(page.envelope().row_count(), 1, "PostgreSQL {tag}");
    assert_eq!(page.envelope().delivery(), PageDelivery::Final);
    assert!(
        page.envelope()
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
    assert_eq!(page.cell(0, 6).unwrap().kind(), ValueKind::Unknown);
    assert_eq!(page.cell(0, 7).unwrap().kind(), ValueKind::Text);
    assert_eq!(page.cell(0, 7).unwrap().bytes(), "éééé".as_bytes());
    assert_eq!(page.cell(0, 8).unwrap().kind(), ValueKind::Binary);
    assert_eq!(page.cell(0, 8).unwrap().bytes(), &[0, 1, 255]);
    assert_eq!(page.cell(0, 9).unwrap().kind(), ValueKind::Unknown);
    assert!(page.cell(0, 9).unwrap().bytes().len() <= 8);
    assert_eq!(page.cell(0, 10).unwrap().kind(), ValueKind::Unknown);
    assert!(page.cell(0, 11).unwrap().is_null());
    assert!(stream.next_page(identity(), 1).await.unwrap().is_none());
    drop(stream);

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
    assert_eq!(page.cell(0, 5).unwrap().kind(), ValueKind::Unknown);
    assert_eq!(page.columns()[5].engine_type().name(), "_int4");
    assert!(!page.cell(0, 5).unwrap().bytes().is_empty());
    assert!(parameters.next_page(identity(), 1).await.unwrap().is_none());
    drop(parameters);
    session.shutdown().await.unwrap();
}
