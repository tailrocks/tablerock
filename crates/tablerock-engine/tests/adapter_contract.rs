use tablerock_core::{BoundedText, ByteLimit, Engine, PageLimits};
use tablerock_engine::{
    AdapterError, AdapterFailureClass, ClickHouseProbeQuery, DriverPageRequest, PostgresProbeQuery,
};

fn text(value: &str) -> BoundedText {
    BoundedText::copy_from_str(value, ByteLimit::new(128)).unwrap()
}

#[test]
fn typed_requests_preserve_engine_identity_and_redact_query_ids() {
    let postgres = DriverPageRequest::PostgreSqlProbe {
        query: PostgresProbeQuery::BoundedSeries,
        limits: PageLimits::new(2, 8, 256, 64),
        max_cell_bytes: 32,
    };
    assert_eq!(postgres.engine(), Engine::PostgreSql);

    let clickhouse = DriverPageRequest::ClickHouseProbe {
        query: ClickHouseProbeQuery::TypedValues,
        query_id: text("private-correlation-value"),
        limits: PageLimits::new(2, 8, 256, 64),
        max_cell_bytes: 32,
    };
    assert_eq!(clickhouse.engine(), Engine::ClickHouse);
    let debug = format!("{clickhouse:?}");
    assert!(!debug.contains("private-correlation-value"));
    assert!(debug.contains("query_id_bytes"));
}

#[test]
fn adapter_errors_expose_only_engine_and_safe_class() {
    let error = AdapterError::new(Engine::Redis, AdapterFailureClass::Protocol);
    assert_eq!(error.engine(), Engine::Redis);
    assert_eq!(error.class(), AdapterFailureClass::Protocol);
    assert_eq!(
        error.to_string(),
        "Redis adapter operation failed (Protocol)"
    );
}
