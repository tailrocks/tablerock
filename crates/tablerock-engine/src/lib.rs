//! Database adapters and operation runtime for TableRock.

mod adapter;
mod clickhouse;
mod postgres;
mod redis;
mod runtime;
mod secret_resolution;
mod service;
mod session_pool;
mod temporal;

pub use adapter::{
    AdapterError, AdapterFailureClass, DriverFuture, DriverPageRequest, DriverPageStream,
    DriverSession, SessionHealth,
};
pub use clickhouse::{
    ClickHouseCompression, ClickHouseConnectConfig, ClickHouseError, ClickHouseProbeQuery,
    ClickHouseRowStream, ClickHouseSession, ClickHouseTlsMode,
};
pub use postgres::{
    PostgresCancellationOutcome, PostgresClientIdentity, PostgresConnectConfig, PostgresCopyChunk,
    PostgresCopyLimits, PostgresCopyOutStream, PostgresCopyOutcome, PostgresError, PostgresNotice,
    PostgresNoticeDelivery, PostgresProbeQuery, PostgresRowStream, PostgresSession,
    PostgresStatementKind, PostgresStatementOutcome, PostgresTlsMaterial, PostgresTlsMode,
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
pub use secret_resolution::{
    ResolvedSecret, SecretPromptPort, SecretResolutionError, SecretSourceKindLabel,
    resolve_for_connect,
};
pub use service::{
    EngineCancelOutcome, EngineService, EngineServiceError, EngineServiceUpdate,
    EngineShutdownOutcome,
};
pub use session_pool::{
    MAX_REGISTERED_SESSIONS, SessionRegistry, SessionRegistryError, SessionSlot,
};
