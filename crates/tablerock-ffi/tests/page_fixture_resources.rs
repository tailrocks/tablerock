use std::path::PathBuf;

use tablerock_core::{
    BoundedText, ByteLimit, ColumnMetadata, Engine, EngineType, IdParts, OwnedValue, PageDelivery,
    PageFacts, PageIdentity, PageLimits, PageWarnings, ResultId, ResultPage, Revision, RowTotal,
};

#[test]
fn committed_swift_page_fixtures_match_current_rust_encoder() {
    for (file, engine, low, type_name, value) in [
        (
            "postgres-signed-v1.hex",
            Engine::PostgreSql,
            41,
            "int8",
            -42,
        ),
        (
            "clickhouse-signed-v1.hex",
            Engine::ClickHouse,
            42,
            "Int64",
            7,
        ),
        ("redis-signed-v1.hex", Engine::Redis, 43, "integer", 99),
    ] {
        let expected = page(engine, low, type_name, value).encode_v1();
        let fixture = fixture_bytes(file);
        assert_eq!(fixture, expected, "fixture drift: {file}");
    }
}

fn page(engine: Engine, low: u64, type_name: &str, value: i64) -> ResultPage {
    let result_id = ResultId::from_parts(IdParts::new(0, low).unwrap()).unwrap();
    ResultPage::from_row_major(
        PageIdentity::new(result_id, Revision::INITIAL, engine),
        0,
        RowTotal::Known(1),
        PageFacts::new(PageDelivery::Final, PageWarnings::none()),
        vec![ColumnMetadata::new(
            BoundedText::copy_from_str("n", ByteLimit::new(1)).unwrap(),
            EngineType::new(
                engine,
                BoundedText::copy_from_str(type_name, ByteLimit::new(16)).unwrap(),
            )
            .unwrap(),
            false,
        )],
        vec![OwnedValue::signed(value)],
        PageLimits::new(500, 64, 1024 * 1024, 64 * 1024),
    )
    .unwrap()
}

fn fixture_bytes(file: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../native/Tests/TableRockBridgeTests/Fixtures/PageV1")
        .join(file);
    let hex = std::fs::read_to_string(path).unwrap();
    let compact: Vec<u8> = hex
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    assert_eq!(compact.len() % 2, 0);
    compact
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).unwrap();
            u8::from_str_radix(text, 16).unwrap()
        })
        .collect()
}
