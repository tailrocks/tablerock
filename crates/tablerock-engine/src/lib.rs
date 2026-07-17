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
    RedisBlockingPopStream, RedisCancelDispatch, RedisClientIdentity, RedisCollectionScanKind,
    RedisCollectionScanOptions, RedisCollectionStream, RedisConnectConfig, RedisConnectionSecurity,
    RedisCredentials, RedisError, RedisKeyStream, RedisProtocol, RedisRuntimePolicy, RedisSession,
    RedisSubscriptionKind, RedisSubscriptionOptions, RedisSubscriptionStream, RedisTlsMaterial,
    RedisTlsMode, RedisTtlApplication, RedisTtlMutationOutcome,
};
pub use runtime::{
    DriverOperationEvent, DriverOperationEvents, DriverRuntime, DriverRuntimeError,
    DriverSpawnError, DriverTaskExit, RuntimeCancelOutcome, RuntimeStopOutcome,
};
pub use service::{
    EngineCancelOutcome, EngineService, EngineServiceError, EngineServiceUpdate,
    EngineShutdownOutcome,
};
