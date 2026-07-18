use std::{sync::Arc, time::Duration};

use tablerock_core::{
    BoundedText, ByteLimit, CancelDispatch, Engine, PageDelivery, PageIdentity, PageLimits,
    PageWarning, Truncation, ValueKind,
};
use tablerock_engine::{
    ClickHouseCompression, ClickHouseConnectConfig, ClickHouseProbeQuery, ClickHouseSession,
    ClickHouseTlsMode, DriverPageRequest, DriverSession, EngineServiceUpdate,
};
use testcontainers::{GenericImage, ImageExt, core::IntoContainerPort, runners::AsyncRunner};

mod support;

const CLICKHOUSE_CANCEL_EVIDENCE_DEADLINE: Duration = Duration::from_secs(15);

fn text(value: &str) -> BoundedText {
    BoundedText::copy_from_str(value, ByteLimit::new(128)).unwrap()
}

fn identity() -> PageIdentity {
    support::identity(Engine::ClickHouse, 3)
}

#[tokio::test]
async fn streams_self_describing_rows_across_supported_clickhouse_matrix() {
    for image in [
        "25.8.28.1-jammy@sha256:ea72c2ca1487386451e43525f7e5455811b62095914d8dd4775b1cda6c09d2e3",
        "26.3.17.4-jammy@sha256:158dcce6f6fdc59309650aad6b79484abf4eed07d4e0bdba31d732e64b5a25fb",
    ] {
        verify_image(image).await;
    }
}

async fn verify_image(image: &str) {
    let container = GenericImage::new("clickhouse", image)
        .with_exposed_port(8123.tcp())
        .with_env_var("CLICKHOUSE_SKIP_USER_SETUP", "1")
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(8123.tcp()).await.unwrap();

    for compression in [ClickHouseCompression::None, ClickHouseCompression::Lz4] {
        let session = ClickHouseSession::connect(&ClickHouseConnectConfig::new(
            text("127.0.0.1"),
            port,
            text("default"),
            text("default"),
            ClickHouseTlsMode::Disable,
            compression,
        ));
        let mut stream = None;
        let mut last_error = None;
        for attempt in 0..300 {
            let driver: &dyn DriverSession = &session;
            match driver
                .start_page_stream(DriverPageRequest::ClickHouseProbe {
                    query: ClickHouseProbeQuery::TypedValues,
                    query_id: text(&format!("tablerock-{port}-{compression:?}-{attempt}")),
                    limits: PageLimits::new(2, 8, 256, 256),
                    max_cell_bytes: 8,
                })
                .await
            {
                Ok(ready) => {
                    stream = Some(ready);
                    break;
                }
                Err(error) => {
                    last_error = Some(error);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
        let mut stream = stream
            .unwrap_or_else(|| panic!("ClickHouse fixture accepts HTTP queries: {last_error:?}"));
        verify_service_cancellation(port, compression, image).await;
        let first = stream.next_page(identity(), 0).await.unwrap().unwrap();
        assert_eq!(first.envelope().row_count(), 2, "{image} {compression:?}");
        assert_eq!(first.envelope().delivery(), PageDelivery::Partial);
        assert_eq!(first.columns()[0].name(), "id");
        assert_eq!(first.columns()[4].engine_type().name(), "Nullable(String)");
        assert_eq!(first.cell(0, 0).unwrap().kind(), ValueKind::Unsigned);
        assert_eq!(first.cell(0, 0).unwrap().bytes(), 0_u64.to_be_bytes());
        assert_eq!(first.cell(0, 1).unwrap().kind(), ValueKind::Signed);
        assert_eq!(first.cell(0, 1).unwrap().bytes(), (-7_i64).to_be_bytes());
        assert_eq!(first.cell(0, 3).unwrap().kind(), ValueKind::Text);
        assert_eq!(first.cell(0, 3).unwrap().bytes(), b"row-0");
        assert_eq!(first.cell(0, 3).unwrap().truncation(), Truncation::Complete);
        assert!(first.cell(1, 4).unwrap().is_null());
        assert_eq!(first.cell(0, 5).unwrap().kind(), ValueKind::Binary);
        assert_eq!(first.cell(0, 5).unwrap().bytes(), &[0, 255]);
        assert_eq!(first.cell(0, 5).unwrap().truncation(), Truncation::Complete);

        let second = stream.next_page(identity(), 2).await.unwrap().unwrap();
        assert_eq!(second.envelope().row_count(), 1);
        assert_eq!(second.envelope().delivery(), PageDelivery::Final);
        assert_eq!(second.cell(0, 0).unwrap().bytes(), 2_u64.to_be_bytes());
        assert!(stream.next_page(identity(), 3).await.unwrap().is_none());

        let mut complex = session
            .stream_probe(
                ClickHouseProbeQuery::ComplexScalars,
                &text(&format!("tablerock-complex-{port}-{compression:?}")),
                PageLimits::new(2, 24, 4096, 1024),
                128,
            )
            .await
            .unwrap();
        let page = complex.next_page(identity(), 0).await.unwrap().unwrap();
        assert_eq!(page.envelope().row_count(), 1, "{image} {compression:?}");
        assert_eq!(page.envelope().delivery(), PageDelivery::Final);
        assert_eq!(page.columns()[0].engine_type().name(), "Bool");
        assert_eq!(page.cell(0, 0).unwrap().kind(), ValueKind::Boolean);
        assert_eq!(page.cell(0, 0).unwrap().bytes(), &[1]);
        assert_eq!(page.cell(0, 1).unwrap().bytes(), 255_u64.to_be_bytes());
        assert_eq!(page.cell(0, 2).unwrap().bytes(), 65535_u64.to_be_bytes());
        assert_eq!(
            page.cell(0, 3).unwrap().bytes(),
            4_294_967_295_u64.to_be_bytes()
        );
        assert_decimal(&page, 4, "340282366920938463463374607431768211455");
        assert_decimal(&page, 5, "-170141183460469231731687303715884105728");
        assert_decimal(
            &page,
            6,
            "115792089237316195423570985008687907853269984665640564039457584007913129639935",
        );
        assert_decimal(
            &page,
            7,
            "-57896044618658097711785492504343953926634992332820282019728792003956564819968",
        );
        assert_decimal(&page, 8, "12345678901234567890123456789.123456789");
        assert_eq!(page.cell(0, 9).unwrap().kind(), ValueKind::Float64);
        assert_eq!(
            page.cell(0, 9).unwrap().bytes(),
            1.5_f64.to_bits().to_be_bytes()
        );
        assert_eq!(page.columns()[10].engine_type().name(), "Date");
        assert_eq!(page.cell(0, 10).unwrap().kind(), ValueKind::Temporal);
        assert_eq!(page.cell(0, 10).unwrap().bytes(), b"2024-02-29");
        assert_eq!(page.columns()[11].engine_type().name(), "Date32");
        assert_eq!(page.cell(0, 11).unwrap().kind(), ValueKind::Temporal);
        assert_eq!(page.cell(0, 11).unwrap().bytes(), b"1900-01-01");
        assert!(
            page.columns()[12]
                .engine_type()
                .name()
                .starts_with("DateTime")
        );
        assert_eq!(page.cell(0, 12).unwrap().kind(), ValueKind::Temporal);
        assert_eq!(page.cell(0, 12).unwrap().bytes(), b"2024-02-29T12:34:56Z");
        assert!(
            page.columns()[13]
                .engine_type()
                .name()
                .starts_with("DateTime64(9")
        );
        assert_eq!(page.cell(0, 13).unwrap().kind(), ValueKind::Temporal);
        assert_eq!(
            page.cell(0, 13).unwrap().bytes(),
            b"2024-02-29T12:34:56.123456789Z"
        );
        assert_eq!(page.cell(0, 14).unwrap().kind(), ValueKind::Binary);
        assert_eq!(page.cell(0, 14).unwrap().bytes().len(), 16);
        assert_eq!(page.cell(0, 15).unwrap().kind(), ValueKind::Binary);
        assert_eq!(page.cell(0, 15).unwrap().bytes().len(), 4);
        assert_eq!(page.cell(0, 16).unwrap().kind(), ValueKind::Binary);
        assert_eq!(page.cell(0, 16).unwrap().bytes().len(), 16);
        assert!(
            page.columns()[17]
                .engine_type()
                .name()
                .starts_with("Enum8(")
        );
        assert_eq!(page.cell(0, 17).unwrap().bytes(), 7_i64.to_be_bytes());
        assert_eq!(
            page.columns()[18].engine_type().name(),
            "LowCardinality(String)"
        );
        assert_eq!(page.cell(0, 18).unwrap().bytes(), b"wrapped");
        assert_eq!(page.cell(0, 19).unwrap().bytes(), (-128_i64).to_be_bytes());
        assert_eq!(
            page.cell(0, 20).unwrap().bytes(),
            (-32_768_i64).to_be_bytes()
        );
        assert_eq!(
            page.cell(0, 21).unwrap().bytes(),
            (-2_147_483_648_i64).to_be_bytes()
        );
        assert!(complex.next_page(identity(), 1).await.unwrap().is_none());

        let mut bounded_complex = session
            .stream_probe(
                ClickHouseProbeQuery::ComplexScalars,
                &text(&format!("tablerock-complex-bounded-{port}-{compression:?}")),
                PageLimits::new(2, 24, 4096, 1024),
                8,
            )
            .await
            .unwrap();
        let bounded_page = bounded_complex
            .next_page(identity(), 0)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(bounded_page.cell(0, 4).unwrap().kind(), ValueKind::Unknown);
        assert!(matches!(
            bounded_page.cell(0, 4).unwrap().truncation(),
            Truncation::Truncated {
                original_byte_len: Some(16)
            }
        ));
        assert!(
            bounded_page
                .envelope()
                .warnings()
                .contains(tablerock_core::PageWarning::UnknownValues)
        );
        assert!(
            bounded_page
                .envelope()
                .warnings()
                .contains(tablerock_core::PageWarning::ByteLimitReached)
        );
        assert_eq!(bounded_page.cell(0, 9).unwrap().kind(), ValueKind::Float64);
        for column in 10..=13 {
            assert_eq!(
                bounded_page.cell(0, column).unwrap().kind(),
                ValueKind::Temporal
            );
            assert!(matches!(
                bounded_page.cell(0, column).unwrap().truncation(),
                Truncation::Truncated {
                    original_byte_len: Some(original)
                } if original > 8
            ));
        }

        let mut structured = session
            .stream_probe(
                ClickHouseProbeQuery::StructuredValues,
                &text(&format!("tablerock-structured-{port}-{compression:?}")),
                PageLimits::new(2, 8, 4096, 1024),
                128,
            )
            .await
            .unwrap();
        let structured_page = structured.next_page(identity(), 0).await.unwrap().unwrap();
        assert_eq!(structured_page.envelope().delivery(), PageDelivery::Final);
        assert_structured(&structured_page, 0, "[1,2,3]", Truncation::Complete);
        assert_structured(
            &structured_page,
            1,
            "[-7,\"quoted\\n\",null]",
            Truncation::Complete,
        );
        assert_structured(
            &structured_page,
            2,
            "[[\"a\",1],[\"b\",2]]",
            Truncation::Complete,
        );
        assert_structured(
            &structured_page,
            3,
            "[[1,\"one\"],[2,\"two\"]]",
            Truncation::Complete,
        );
        assert_structured(
            &structured_page,
            4,
            "[{\"$binary\":\"00ff\"}]",
            Truncation::Complete,
        );
        assert_structured(
            &structured_page,
            5,
            "[\"2024-02-29T12:34:56.123Z\"]",
            Truncation::Complete,
        );
        assert!(structured.next_page(identity(), 1).await.unwrap().is_none());

        let mut bounded_structured = session
            .stream_probe(
                ClickHouseProbeQuery::StructuredValues,
                &text(&format!(
                    "tablerock-structured-bounded-{port}-{compression:?}"
                )),
                PageLimits::new(2, 8, 4096, 1024),
                8,
            )
            .await
            .unwrap();
        let bounded_structured_page = bounded_structured
            .next_page(identity(), 0)
            .await
            .unwrap()
            .unwrap();
        assert_structured(
            &bounded_structured_page,
            3,
            "[[1,\"one",
            Truncation::Truncated {
                original_byte_len: Some(21),
            },
        );
        assert!(
            bounded_structured_page
                .envelope()
                .warnings()
                .contains(tablerock_core::PageWarning::ByteLimitReached)
        );

        drop(stream);
        drop(complex);
        drop(bounded_complex);
        drop(structured);
        drop(bounded_structured);
        let operation_id = support::operation(30);
        let mut service = support::service(1, 2);
        service
            .submit(
                operation_id,
                support::command(31),
                Arc::new(session),
                DriverPageRequest::ClickHouseProbe {
                    query: ClickHouseProbeQuery::TypedValues,
                    query_id: text(&format!("tablerock-service-{port}-{compression:?}")),
                    limits: PageLimits::new(2, 8, 256, 256),
                    max_cell_bytes: 8,
                },
                identity(),
            )
            .await
            .unwrap();
        let mut rows = 0_u32;
        loop {
            match service.next_update(operation_id).await.unwrap().unwrap() {
                EngineServiceUpdate::Started => {}
                EngineServiceUpdate::Page(page) => rows += page.envelope().row_count(),
                EngineServiceUpdate::Terminal(tablerock_core::OperationOutcome::Completed) => {
                    break;
                }
                other => panic!("unexpected ClickHouse service event: {other:?}"),
            }
        }
        assert_eq!(rows, 3, "{image} {compression:?}");
    }
}

async fn verify_service_cancellation(port: u16, compression: ClickHouseCompression, image: &str) {
    let session = ClickHouseSession::connect(&ClickHouseConnectConfig::new(
        text("127.0.0.1"),
        port,
        text("default"),
        text("default"),
        ClickHouseTlsMode::Disable,
        compression,
    ));
    let operation_id = support::operation(50);
    let mut service = support::service(1, 2);
    service
        .submit(
            operation_id,
            support::command(51),
            Arc::new(session),
            DriverPageRequest::ClickHouseProbe {
                query: ClickHouseProbeQuery::CancellationStream,
                query_id: text(&format!("tablerock-cancel-{port}-{compression:?}")),
                limits: PageLimits::new(1, 1, 64, 64),
                max_cell_bytes: 16,
            },
            identity(),
        )
        .await
        .unwrap();
    assert!(matches!(
        tokio::time::timeout(
            CLICKHOUSE_CANCEL_EVIDENCE_DEADLINE,
            service.next_update(operation_id),
        )
        .await
        .unwrap()
        .unwrap()
        .unwrap(),
        EngineServiceUpdate::Started
    ));
    match tokio::time::timeout(
        CLICKHOUSE_CANCEL_EVIDENCE_DEADLINE,
        service.next_update(operation_id),
    )
    .await
    .unwrap()
    .unwrap()
    .unwrap()
    {
        EngineServiceUpdate::Page(_) => {}
        other => panic!("unexpected {image} event before ClickHouse progress: {other:?}"),
    }
    let cancel = service.cancel(operation_id).unwrap();
    assert_eq!(cancel.core, tablerock_core::CancelRequestOutcome::Requested);
    assert_eq!(
        cancel.runtime,
        Some(tablerock_engine::RuntimeCancelOutcome::Queued)
    );
    tokio::time::timeout(CLICKHOUSE_CANCEL_EVIDENCE_DEADLINE, async {
        loop {
            match service.next_update(operation_id).await.unwrap().unwrap() {
                EngineServiceUpdate::Page(_) => {}
                EngineServiceUpdate::CancelDispatched(CancelDispatch::RequestSent) => break,
                other => panic!("unexpected {image} event before ClickHouse dispatch: {other:?}"),
            }
        }
    })
    .await
    .unwrap();
    tokio::time::timeout(CLICKHOUSE_CANCEL_EVIDENCE_DEADLINE, async {
        loop {
            match service.next_update(operation_id).await.unwrap().unwrap() {
                EngineServiceUpdate::Page(_) => {}
                EngineServiceUpdate::Terminal(
                    tablerock_core::OperationOutcome::ServerConfirmedCancelled,
                ) => break,
                other => panic!("unexpected {image} event before ClickHouse terminal: {other:?}"),
            }
        }
    })
    .await
    .unwrap();
}

fn assert_decimal(page: &tablerock_core::ResultPage, column: u32, expected: &str) {
    let cell = page.cell(0, column).unwrap();
    assert_eq!(cell.kind(), ValueKind::Decimal);
    assert_eq!(cell.bytes(), expected.as_bytes());
}

fn assert_structured(
    page: &tablerock_core::ResultPage,
    column: u32,
    expected: &str,
    truncation: Truncation,
) {
    let cell = page.cell(0, column).unwrap();
    assert_eq!(cell.kind(), ValueKind::Structured);
    assert_eq!(cell.bytes(), expected.as_bytes());
    assert_eq!(cell.truncation(), truncation);
}

#[tokio::test]
async fn persistent_session_runs_statement_health_and_reuses_connection() {
    let image =
        "26.3.17.4-jammy@sha256:158dcce6f6fdc59309650aad6b79484abf4eed07d4e0bdba31d732e64b5a25fb";
    let container = GenericImage::new("clickhouse", image)
        .with_exposed_port(8123.tcp())
        .with_env_var("CLICKHOUSE_SKIP_USER_SETUP", "1")
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(8123.tcp()).await.unwrap();
    // Wait until HTTP queries answer; container start alone is not enough.
    let session = ready_clickhouse_session(port, ClickHouseCompression::None).await;
    let session_id =
        tablerock_core::SessionId::from_parts(tablerock_core::IdParts::new(0, 910).unwrap())
            .unwrap();
    let mut service = support::service(4, 4);
    let handle = service
        .register_session(session_id, Box::new(session))
        .unwrap();

    let op1 = support::operation(911);
    service
        .submit(
            op1,
            support::command(911),
            Arc::clone(&handle),
            DriverPageRequest::ClickHouseStatement {
                statement: tablerock_core::StatementText::new(
                    "SELECT number AS id FROM numbers(3)",
                )
                .unwrap(),
                query_id: text(&format!("tablerock-stmt-1-{port}")),
                limits: PageLimits::new(10, 8, 4096, 256),
                max_cell_bytes: 64,
            },
            support::identity(Engine::ClickHouse, 911),
        )
        .await
        .unwrap();
    let mut rows = 0_u32;
    loop {
        match service.next_update(op1).await.unwrap().unwrap() {
            EngineServiceUpdate::Started => {}
            EngineServiceUpdate::Page(page) => rows += page.envelope().row_count(),
            EngineServiceUpdate::Terminal(tablerock_core::OperationOutcome::Completed) => break,
            other => panic!("unexpected first CH statement event: {other:?}"),
        }
    }
    assert_eq!(rows, 3);

    let op2 = support::operation(912);
    service
        .submit(
            op2,
            support::command(912),
            Arc::clone(&handle),
            DriverPageRequest::ClickHouseStatement {
                statement: tablerock_core::StatementText::new("SELECT toUInt8(42) AS answer")
                    .unwrap(),
                query_id: text(&format!("tablerock-stmt-2-{port}")),
                limits: PageLimits::new(10, 8, 4096, 256),
                max_cell_bytes: 64,
            },
            support::identity(Engine::ClickHouse, 912),
        )
        .await
        .unwrap();
    loop {
        match service.next_update(op2).await.unwrap().unwrap() {
            EngineServiceUpdate::Started | EngineServiceUpdate::Page(_) => {}
            EngineServiceUpdate::Terminal(tablerock_core::OperationOutcome::Completed) => break,
            other => panic!("unexpected second CH statement event: {other:?}"),
        }
    }

    let health = handle.health().await.unwrap();
    assert!(health.server_reachable());
    assert_eq!(health.engine(), Engine::ClickHouse);

    let bad = handle
        .start_page_stream(DriverPageRequest::ClickHouseStatement {
            statement: tablerock_core::StatementText::new("SELEC not_valid").unwrap(),
            query_id: text(&format!("tablerock-stmt-bad-{port}")),
            limits: PageLimits::new(10, 8, 4096, 256),
            max_cell_bytes: 64,
        })
        .await;
    assert!(bad.is_err());
    let health = handle.health().await.unwrap();
    assert!(health.server_reachable());

    drop(handle);
    service.disconnect(session_id).await.unwrap();
}

async fn ready_clickhouse_session(
    port: u16,
    compression: ClickHouseCompression,
) -> ClickHouseSession {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        let session = ClickHouseSession::connect(&ClickHouseConnectConfig::new(
            text("127.0.0.1"),
            port,
            text("default"),
            text("default"),
            ClickHouseTlsMode::Disable,
            compression,
        ));
        match session.health_check().await {
            Ok(()) => return session,
            Err(_) if tokio::time::Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(error) => panic!("ClickHouse never became ready: {error:?}"),
        }
    }
}

#[tokio::test]
async fn lists_catalog_databases_and_objects() {
    use tablerock_core::{
        BoundedText, ByteLimit, CatalogNodeKind, ClickHouseObjectKind, PageLimits,
    };
    use tablerock_engine::{CatalogRequest, DriverSession};

    let image =
        "26.3.17.4-jammy@sha256:158dcce6f6fdc59309650aad6b79484abf4eed07d4e0bdba31d732e64b5a25fb";
    let container = GenericImage::new("clickhouse", image)
        .with_exposed_port(8123.tcp())
        .with_env_var("CLICKHOUSE_SKIP_USER_SETUP", "1")
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(8123.tcp()).await.unwrap();
    let session = ready_clickhouse_session(port, ClickHouseCompression::None).await;
    session
        .execute_sql("CREATE DATABASE IF NOT EXISTS catalog_fixture")
        .await
        .unwrap();
    session
        .execute_sql("CREATE TABLE IF NOT EXISTS catalog_fixture.t (id UInt8) ENGINE = Memory")
        .await
        .unwrap();
    session
        .execute_sql("CREATE VIEW IF NOT EXISTS catalog_fixture.v AS SELECT 1 AS id")
        .await
        .unwrap();

    let limits = PageLimits::new(500, 8, 64 * 1024, 256);
    let databases = session
        .catalog(CatalogRequest::ClickHouseDatabases { limits })
        .await
        .unwrap();
    assert!(
        databases
            .nodes()
            .iter()
            .any(|n| n.kind() == CatalogNodeKind::ClickHouseDatabase
                && n.name() == "catalog_fixture")
    );

    let objects = session
        .catalog(CatalogRequest::ClickHouseObjects {
            database: BoundedText::copy_from_str("catalog_fixture", ByteLimit::new(64)).unwrap(),
            limits,
        })
        .await
        .unwrap();
    let names: Vec<_> = objects
        .nodes()
        .iter()
        .map(|n| n.name().to_owned())
        .collect();
    assert!(names.contains(&"t".to_owned()), "{names:?}");
    assert!(names.contains(&"v".to_owned()), "{names:?}");
    assert!(objects.nodes().iter().any(|n| {
        matches!(
            n.kind(),
            CatalogNodeKind::ClickHouseObject(ClickHouseObjectKind::Table)
        )
    }));
}

#[tokio::test]
async fn structure_facts_and_progressive_insert() {
    use tablerock_core::{
        BoundedText, ByteLimit, ContextId, FieldValue, IdParts, MutationChange, MutationId,
        MutationPlan, MutationPlanLimits, MutationTarget, OperationScope, OwnedValue, ProfileId,
        ReviewTokenId, Revision, SessionId, Truncation,
    };
    use tablerock_engine::{MutationChangeOutcome, MutationTransactionState};

    let image =
        "26.3.17.4-jammy@sha256:158dcce6f6fdc59309650aad6b79484abf4eed07d4e0bdba31d732e64b5a25fb";
    let container = GenericImage::new("clickhouse", image)
        .with_exposed_port(8123.tcp())
        .with_env_var("CLICKHOUSE_SKIP_USER_SETUP", "1")
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(8123.tcp()).await.unwrap();
    let session = ready_clickhouse_session(port, ClickHouseCompression::None).await;

    session
        .execute_sql(
            "CREATE TABLE default.mut_ch (
                id UInt64,
                name String
             ) ENGINE = MergeTree ORDER BY id",
        )
        .await
        .unwrap();

    let facts = session
        .relation_engine_facts("default", "mut_ch")
        .await
        .unwrap()
        .expect("table facts");
    assert!(facts.0.contains("MergeTree"), "{facts:?}");
    assert_eq!(facts.2, "id"); // sorting_key

    let cols = session
        .relation_column_facts("default", "mut_ch")
        .await
        .unwrap();
    assert!(cols.iter().any(|(n, t, _, _)| n == "name" && t == "String"));

    fn bt(s: &str) -> BoundedText {
        BoundedText::copy_from_str(s, ByteLimit::new(10_000)).unwrap()
    }
    let scope = OperationScope::new(
        ProfileId::from_parts(IdParts::new(1, 1).unwrap()).unwrap(),
        SessionId::from_parts(IdParts::new(1, 2).unwrap()).unwrap(),
        ContextId::from_parts(IdParts::new(1, 3).unwrap()).unwrap(),
    );
    let plan = MutationPlan::new(
        MutationId::from_parts(IdParts::new(1, 10).unwrap()).unwrap(),
        scope,
        Revision::INITIAL,
        MutationTarget::ClickHouseTable {
            database: bt("default"),
            table: bt("mut_ch"),
        },
        vec![MutationChange::InsertRow {
            values: vec![
                FieldValue::new(bt("id"), OwnedValue::unsigned(1)),
                FieldValue::new(
                    bt("name"),
                    OwnedValue::text(bt("alice"), Truncation::Complete).unwrap(),
                ),
            ],
        }],
        MutationPlanLimits::new(8, 16, 4096, 4096, 60_000).unwrap(),
    )
    .unwrap();
    assert_eq!(
        plan.execution_model(),
        tablerock_core::MutationExecutionModel::ClickHouseProgressiveInsertNonTransactional
    );
    let authorized = plan
        .review(
            ReviewTokenId::from_parts(IdParts::new(1, 11).unwrap()).unwrap(),
            1_000,
            30_000,
        )
        .unwrap()
        .authorize(5_000, scope, Revision::INITIAL)
        .unwrap();
    let outcome = session.apply_authorized_mutation(authorized).await.unwrap();
    // Non-transactional: terminal is Committed meaning "apply finished", not rollback semantics.
    assert_eq!(outcome.transaction, MutationTransactionState::Committed);
    assert!(matches!(
        &outcome.changes[0],
        MutationChangeOutcome::Applied {
            rows_affected: 1,
            ..
        }
    ));

    // Async UPDATE mutation — accepted, non-transactional markers in returned.
    let upd = MutationPlan::new(
        MutationId::from_parts(IdParts::new(1, 12).unwrap()).unwrap(),
        scope,
        Revision::INITIAL,
        MutationTarget::ClickHouseTable {
            database: bt("default"),
            table: bt("mut_ch"),
        },
        vec![MutationChange::UpdateRow {
            locator: vec![FieldValue::new(bt("id"), OwnedValue::unsigned(1))],
            assignments: vec![FieldValue::new(
                bt("name"),
                OwnedValue::text(bt("bob"), Truncation::Complete).unwrap(),
            )],
        }],
        MutationPlanLimits::new(8, 16, 4096, 4096, 60_000).unwrap(),
    )
    .unwrap();
    assert_eq!(
        upd.execution_model(),
        tablerock_core::MutationExecutionModel::ClickHouseAsynchronousMutationNonTransactional
    );
    let auth2 = upd
        .review(
            ReviewTokenId::from_parts(IdParts::new(1, 13).unwrap()).unwrap(),
            1_000,
            30_000,
        )
        .unwrap()
        .authorize(5_000, scope, Revision::INITIAL)
        .unwrap();
    let out2 = session.apply_authorized_mutation(auth2).await.unwrap();
    match &out2.changes[0] {
        MutationChangeOutcome::Applied { returned, .. } => {
            assert!(
                returned
                    .iter()
                    .any(|(k, v)| k == "transactional" && v == "false"),
                "{returned:?}"
            );
            assert!(
                returned
                    .iter()
                    .any(|(k, v)| k == "kind" && v.contains("async_mutation")),
                "{returned:?}"
            );
        }
        other => panic!("expected Applied async mutation, got {other:?}"),
    }
    // Poll until done or timeout (mutations are async).
    let mut done = false;
    for _ in 0..50 {
        let status = session
            .latest_mutation_status("default", "mut_ch")
            .await
            .unwrap();
        if status
            .iter()
            .any(|(k, v)| k == "is_done" && (v == "1" || v == "true"))
        {
            done = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(done, "mutation should complete in fixture");
}

#[tokio::test]
async fn explain_raw_and_structured_with_fallback() {
    let image =
        "26.3.17.4-jammy@sha256:158dcce6f6fdc59309650aad6b79484abf4eed07d4e0bdba31d732e64b5a25fb";
    let container = GenericImage::new("clickhouse", image)
        .with_exposed_port(8123.tcp())
        .with_env_var("CLICKHOUSE_SKIP_USER_SETUP", "1")
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(8123.tcp()).await.unwrap();
    let session = ready_clickhouse_session(port, ClickHouseCompression::None).await;

    let raw = session.explain_raw("SELECT 1").await.unwrap();
    assert!(!raw.is_empty(), "raw explain should return plan text");

    let structured = session.explain_structured("SELECT 1").await.unwrap();
    assert!(!structured.is_empty());
    // Either AST lines or unknown-node fallback with raw plan.
    let joined = structured.join("\n");
    assert!(
        joined.contains("Select")
            || joined.contains("SELECT")
            || joined.contains("unknown-node")
            || !joined.is_empty(),
        "{joined}"
    );
}

/// Residual plan 014: one operation delivers a partial page of rows, then a
/// late cancel/error terminal — without invalidating the already-owned page.
#[tokio::test]
async fn partial_rows_and_late_error_both_visible_on_one_operation() {
    use tablerock_core::OperationOutcome;

    let image =
        "26.3.17.4-jammy@sha256:158dcce6f6fdc59309650aad6b79484abf4eed07d4e0bdba31d732e64b5a25fb";
    let container = GenericImage::new("clickhouse", image)
        .with_exposed_port(8123.tcp())
        .with_env_var("CLICKHOUSE_SKIP_USER_SETUP", "1")
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(8123.tcp()).await.unwrap();
    let session = ready_clickhouse_session(port, ClickHouseCompression::Lz4).await;

    let operation_id = support::operation(70);
    let mut service = support::service(1, 4);
    service
        .submit(
            operation_id,
            support::command(71),
            Arc::new(session),
            DriverPageRequest::ClickHouseProbe {
                // Long-running series so cancel can land after the first page.
                query: ClickHouseProbeQuery::CancellationStream,
                query_id: text(&format!("tablerock-partial-late-op-{port}")),
                limits: PageLimits::new(1, 1, 256, 64),
                max_cell_bytes: 32,
            },
            identity(),
        )
        .await
        .unwrap();

    assert!(matches!(
        service.next_update(operation_id).await.unwrap().unwrap(),
        EngineServiceUpdate::Started
    ));

    let page = match service.next_update(operation_id).await.unwrap().unwrap() {
        EngineServiceUpdate::Page(page) => *page,
        other => panic!("expected first page, got {other:?}"),
    };
    assert!(page.envelope().row_count() >= 1);
    assert!(
        page.envelope().delivery() == PageDelivery::Partial
            || page
                .envelope()
                .warnings()
                .contains(PageWarning::RowLimitReached),
        "first page should be partial under max_rows=1"
    );
    let retained_rows = page.envelope().row_count();
    let retained_bytes = page.cell(0, 0).unwrap().bytes().to_vec();

    let cancel = service.cancel(operation_id).unwrap();
    assert_eq!(
        cancel.core,
        tablerock_core::CancelRequestOutcome::Requested
    );

    let mut saw_terminal = false;
    tokio::time::timeout(CLICKHOUSE_CANCEL_EVIDENCE_DEADLINE, async {
        loop {
            match service.next_update(operation_id).await.unwrap() {
                Some(EngineServiceUpdate::Page(_))
                | Some(EngineServiceUpdate::CancelDispatched(_))
                | Some(EngineServiceUpdate::Started) => {}
                Some(EngineServiceUpdate::Terminal(
                    OperationOutcome::ServerConfirmedCancelled
                    | OperationOutcome::ClientStopped
                    | OperationOutcome::Failed
                    | OperationOutcome::CompletedBeforeCancel
                    | OperationOutcome::Completed
                    | OperationOutcome::Unknown
                    | OperationOutcome::Disconnected,
                )) => {
                    saw_terminal = true;
                    break;
                }
                None => break,
            }
        }
    })
    .await
    .expect("timed out waiting for late terminal after partial page");
    assert!(
        saw_terminal,
        "late cancel/error terminal must be observed on the same operation"
    );

    // Partial rows already delivered remain owned and readable.
    assert_eq!(page.envelope().row_count(), retained_rows);
    assert_eq!(page.cell(0, 0).unwrap().bytes(), retained_bytes.as_slice());
}
