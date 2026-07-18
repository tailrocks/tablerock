//! Database adapters and operation runtime for TableRock.

mod adapter;
mod catalog;
mod clickhouse;
mod ident;
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
pub use catalog::{
    CatalogExactness, CatalogNodeSeed, CatalogRequest, CatalogSubtree,
    REDIS_DEFAULT_LOGICAL_DATABASES, ServerDescribe,
};
pub use clickhouse::{
    ClickHouseCompression, ClickHouseConnectConfig, ClickHouseError, ClickHouseProbeQuery,
    ClickHouseRowStream, ClickHouseSession, ClickHouseTlsMode,
};
pub use ident::{QuoteIdentError, qualify_table, quote_ident};
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
// SQL analysis is pure and lives in core; re-export for engine consumers.
pub use tablerock_core::{SqlDialect, StatementSpan, statement_at, statements};
