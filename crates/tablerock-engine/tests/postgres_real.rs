use tablerock_core::{
    BoundedText, ByteLimit, Engine, IdParts, OperationId, PageDelivery, PageIdentity, PageLimits,
    PageWarning, ResultId, Revision, Truncation, ValueKind,
};
use tablerock_engine::{
    AdapterFailureClass, CancelDispatch, ClickHouseProbeQuery, DriverPageRequest, DriverSession,
    PostgresCancellationOutcome, PostgresConnectConfig, PostgresProbeQuery, PostgresSession,
    PostgresTlsMode,
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
        PostgresTlsMode::Disable,
    ))
    .await
    .unwrap();

    assert_eq!(
        session.cancel_sleep_probe().await.unwrap(),
        PostgresCancellationOutcome::ConfirmedByServer
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

fn identity() -> PageIdentity {
    PageIdentity::new(
        ResultId::from_parts(IdParts::new(0, 1).unwrap()).unwrap(),
        Revision::INITIAL,
        tablerock_core::Engine::PostgreSql,
    )
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
        PostgresTlsMode::Disable,
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
    assert_eq!(
        driver
            .cancel(OperationId::from_parts(IdParts::new(0, 9).unwrap()).unwrap())
            .await,
        CancelDispatch::Unsupported
    );
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
    (Box::new(session) as Box<dyn DriverSession>)
        .shutdown()
        .await
        .unwrap();
}

#[tokio::test]
async fn streams_typed_values_from_supported_postgres_lines() {
    for tag in ["17.10-alpine", "18.4-alpine"] {
        verify_typed_values(tag).await;
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
        PostgresTlsMode::Disable,
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
    session.shutdown().await.unwrap();
}
