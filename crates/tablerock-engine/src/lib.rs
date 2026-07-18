//! Database adapters and operation runtime for TableRock.

mod adapter;
mod browse_plan;
mod catalog;
mod clickhouse;
mod clickhouse_mutation;
mod ident;
mod postgres;
mod postgres_mutation;
mod redis;
mod runtime;
mod secret_resolution;
mod service;
mod session_pool;
mod ssh_tunnel;
mod startup_run;
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
pub use browse_plan::{
    BrowsePlan, BrowsePlanError, FilterOperator, FilterValue, RenderedBrowseSql, SortDirection,
    parse_bind_text,
    SortKey, TypedCondition,
};
pub use ident::{QuoteIdentError, qualify_table, quote_ident};
pub use postgres::{
    PostgresCancellationOutcome, PostgresClientIdentity, PostgresConnectConfig, PostgresCopyChunk,
    PostgresCopyLimits, PostgresCopyOutStream, PostgresCopyOutcome, PostgresError, PostgresNotice,
    PostgresNoticeDelivery, PostgresProbeQuery, PostgresRowStream, PostgresSession,
    PostgresStatementKind, PostgresStatementOutcome, PostgresTlsMaterial, PostgresTlsMode,
};
pub use postgres_mutation::{
    MutationApplyOutcome, MutationChangeOutcome, MutationTransactionState,
};
pub use redis::{
    RedisBlockingPopStream, RedisCancelDispatch, RedisClientIdentity, RedisCollectionScanKind,
    RedisCollectionScanOptions, RedisCollectionStream, RedisConnectConfig, RedisConnectionSecurity,
    RedisCredentials, RedisError, RedisInfoSnapshot, RedisKeyStream, RedisPipelineCommand,
    RedisPipelineOutcome, RedisProtocol, RedisRuntimePolicy, RedisSession, RedisStreamEntry,
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
pub use ssh_tunnel::{
    ClientHandler, LocalForwardTunnel, SSH_KEEPALIVE_INTERVAL_SECS, SSH_KEEPALIVE_MAX,
    SshAgentAuth, SshAuthMaterial, SshHostKeyPolicy, SshPasswordAuth, SshPublicKeyAuth,
    SshTunnelConfig, SshTunnelError, channel_stream, connect_session,
    connect_session_capture_host_key, learn_host_key, open_direct_tcpip, open_local_forward_tunnel,
    spawn_local_forward, ssh_client_config,
};
pub use startup_run::{
    run_clickhouse_startup_actions, run_postgres_startup_actions, run_redis_startup_actions,
    run_redis_one_authorized, run_sql_one_ch_authorized, run_sql_one_pg_authorized,
};
// SQL analysis is pure and lives in core; re-export for engine consumers.
pub use tablerock_core::{SqlDialect, StatementSpan, statement_at, statements};
