use std::time::Duration;

use tablerock_core::{
    BoundedText, ByteLimit, CancelDispatch, Engine, PageDelivery, PageIdentity, PageLimits,
    Truncation, ValueKind,
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
                Box::new(session),
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
            Box::new(session),
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
