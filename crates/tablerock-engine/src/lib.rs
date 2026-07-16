//! Database adapters and operation runtime for TableRock.

mod adapter;
mod clickhouse;
mod postgres;
mod redis;
mod runtime;
mod service;

pub use adapter::{
    AdapterError, AdapterFailureClass, DriverFuture, DriverPageRequest, DriverPageStream,
    DriverSession,
};
pub use clickhouse::{
    ClickHouseCompression, ClickHouseConnectConfig, ClickHouseError, ClickHouseProbeQuery,
    ClickHouseRowStream, ClickHouseSession, ClickHouseTlsMode,
};
pub use postgres::{
    PostgresCancellationOutcome, PostgresClientIdentity, PostgresConnectConfig, PostgresError,
    PostgresProbeQuery, PostgresRowStream, PostgresSession, PostgresTlsMaterial, PostgresTlsMode,
};
pub use redis::{
    RedisBlockingPopStream, RedisConnectConfig, RedisError, RedisKeyStream, RedisProtocol,
    RedisSession, RedisTlsMode,
};
pub use runtime::{
    DriverOperationEvent, DriverOperationEvents, DriverRuntime, DriverRuntimeError,
    DriverSpawnError, DriverTaskExit, RuntimeCancelOutcome, RuntimeStopOutcome,
};
pub use service::{
    EngineCancelOutcome, EngineService, EngineServiceError, EngineServiceUpdate,
    EngineShutdownOutcome,
};
