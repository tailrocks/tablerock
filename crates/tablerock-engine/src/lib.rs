//! Database adapters and operation runtime for TableRock.

mod adapter;
mod clickhouse;
mod postgres;
mod redis;
mod runtime;

pub use adapter::{
    AdapterError, AdapterFailureClass, CancelDispatch, DriverFuture, DriverPageRequest,
    DriverPageStream, DriverSession,
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
pub use runtime::{
    DriverOperationEvent, DriverOperationEvents, DriverRuntime, DriverRuntimeError, DriverTaskExit,
    RuntimeCancelOutcome,
};
