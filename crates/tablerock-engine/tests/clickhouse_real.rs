use std::time::Duration;

use tablerock_core::{
    BoundedText, ByteLimit, Engine, IdParts, PageDelivery, PageIdentity, PageLimits, ResultId,
    Revision, Truncation, ValueKind,
};
use tablerock_engine::{
    ClickHouseCompression, ClickHouseConnectConfig, ClickHouseProbeQuery, ClickHouseSession,
    ClickHouseTlsMode,
};
use testcontainers::{GenericImage, ImageExt, core::IntoContainerPort, runners::AsyncRunner};

fn text(value: &str) -> BoundedText {
    BoundedText::copy_from_str(value, ByteLimit::new(128)).unwrap()
}

fn identity() -> PageIdentity {
    PageIdentity::new(
        ResultId::from_parts(IdParts::new(0, 3).unwrap()).unwrap(),
        Revision::INITIAL,
        Engine::ClickHouse,
    )
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
            match session
                .stream_probe(
                    ClickHouseProbeQuery::TypedValues,
                    &text(&format!("tablerock-{port}-{compression:?}-{attempt}")),
                    PageLimits::new(2, 8, 256, 256),
                    8,
                )
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
    }
}
