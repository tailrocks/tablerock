//! Database adapters and operation runtime for TableRock.

mod adapter;
mod clickhouse;
mod postgres;
mod redis;

pub use adapter::{
    AdapterError, AdapterFailureClass, CancelDispatch, DriverFuture, DriverOperationRegistry,
    DriverPageRequest, DriverPageStream, DriverSession, OperationCancelOutcome,
    OperationRegistrationError,
};
pub use clickhouse::{
    ClickHouseCompression, ClickHouseConnectConfig, ClickHouseError, ClickHouseProbeQuery,
    ClickHouseRowStream, ClickHouseSession, ClickHouseTlsMode,
};
pub use postgres::{
    PostgresCancellationOutcome, PostgresConnectConfig, PostgresError, PostgresProbeQuery,
    PostgresRowStream, PostgresSession, PostgresTlsMode,
};
pub use redis::{
    RedisConnectConfig, RedisError, RedisKeyStream, RedisProtocol, RedisSession, RedisTlsMode,
};
