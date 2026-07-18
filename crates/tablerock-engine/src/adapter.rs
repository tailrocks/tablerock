use std::{error::Error, fmt, future::Future, pin::Pin, time::Instant};

use tablerock_core::{
    AuthorizedMutationPlan, BoundedBytes, BoundedText, CancelDispatch, Engine, OperationId,
    PageIdentity, PageLimits, ResultPage, StatementText,
};

use crate::{
    CatalogRequest, CatalogSubtree, ClickHouseError, ClickHouseProbeQuery, ClickHouseRowStream,
    ClickHouseSession, MutationApplyOutcome, PostgresError, PostgresProbeQuery, PostgresRowStream,
    PostgresSession, RedisCollectionScanKind, RedisCollectionScanOptions, RedisCollectionStream,
    RedisError, RedisKeyStream, RedisSession, RedisSubscriptionKind, RedisSubscriptionOptions,
    RedisSubscriptionStream, ServerDescribe,
};

pub type DriverFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Cheap connectivity fact from `DriverSession::health`. No version strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionHealth {
    engine: Engine,
    server_reachable: bool,
    elapsed_millis: u64,
}

impl SessionHealth {
    #[must_use]
    pub const fn new(engine: Engine, server_reachable: bool, elapsed_millis: u64) -> Self {
        Self {
            engine,
            server_reachable,
            elapsed_millis,
        }
    }

    #[must_use]
    pub const fn engine(self) -> Engine {
        self.engine
    }

    #[must_use]
    pub const fn server_reachable(self) -> bool {
        self.server_reachable
    }

    #[must_use]
    pub const fn elapsed_millis(self) -> u64 {
        self.elapsed_millis
    }
}

pub enum DriverPageRequest {
    PostgreSqlProbe {
        query: PostgresProbeQuery,
        limits: PageLimits,
        max_cell_bytes: u64,
    },
    /// Operator-supplied PostgreSQL statement. Text is never logged by Debug.
    PostgreSqlStatement {
        statement: StatementText,
        /// Bound parameters for `$n` placeholders. Never logged by Debug.
        parameters: Vec<crate::browse_plan::FilterValue>,
        limits: PageLimits,
        max_cell_bytes: u64,
    },
    ClickHouseProbe {
        query: ClickHouseProbeQuery,
        query_id: BoundedText,
        limits: PageLimits,
        max_cell_bytes: u64,
    },
    /// Operator-supplied ClickHouse statement. Text is never logged by Debug.
    ClickHouseStatement {
        statement: StatementText,
        query_id: BoundedText,
        limits: PageLimits,
        max_cell_bytes: u64,
    },
    RedisKeyScan {
        limits: PageLimits,
        max_cell_bytes: u64,
        scan_count: u32,
        max_scan_rounds: u32,
        /// Optional SCAN MATCH pattern; None or empty means all keys (COUNT only).
        match_pattern: Option<BoundedBytes>,
    },
    RedisCollectionScan {
        key: BoundedBytes,
        kind: RedisCollectionScanKind,
        options: RedisCollectionScanOptions,
    },
    RedisBlockingPop {
        key: BoundedBytes,
        limits: PageLimits,
        max_cell_bytes: u64,
    },
    RedisSubscribe {
        selector: BoundedBytes,
        kind: RedisSubscriptionKind,
        options: RedisSubscriptionOptions,
    },
}

impl DriverPageRequest {
    #[must_use]
    pub const fn engine(&self) -> Engine {
        match self {
            Self::PostgreSqlProbe { .. } | Self::PostgreSqlStatement { .. } => Engine::PostgreSql,
            Self::ClickHouseProbe { .. } | Self::ClickHouseStatement { .. } => Engine::ClickHouse,
            Self::RedisKeyScan { .. }
            | Self::RedisCollectionScan { .. }
            | Self::RedisBlockingPop { .. }
            | Self::RedisSubscribe { .. } => Engine::Redis,
        }
    }
}

impl fmt::Debug for DriverPageRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("DriverPageRequest");
        debug.field("engine", &self.engine());
        match self {
            Self::PostgreSqlProbe {
                query,
                limits,
                max_cell_bytes,
            } => debug
                .field("probe", query)
                .field("limits", limits)
                .field("max_cell_bytes", max_cell_bytes),
            Self::PostgreSqlStatement {
                statement,
                parameters,
                limits,
                max_cell_bytes,
            } => debug
                .field("statement_bytes", &statement.len())
                .field("parameter_count", &parameters.len())
                .field("limits", limits)
                .field("max_cell_bytes", max_cell_bytes),
            Self::ClickHouseProbe {
                query,
                query_id,
                limits,
                max_cell_bytes,
            } => debug
                .field("probe", query)
                .field("query_id_bytes", &query_id.len())
                .field("limits", limits)
                .field("max_cell_bytes", max_cell_bytes),
            Self::ClickHouseStatement {
                statement,
                query_id,
                limits,
                max_cell_bytes,
            } => debug
                .field("statement_bytes", &statement.len())
                .field("query_id_bytes", &query_id.len())
                .field("limits", limits)
                .field("max_cell_bytes", max_cell_bytes),
            Self::RedisKeyScan {
                limits,
                max_cell_bytes,
                scan_count,
                max_scan_rounds,
                match_pattern,
            } => debug
                .field("limits", limits)
                .field("max_cell_bytes", max_cell_bytes)
                .field("scan_count", scan_count)
                .field("max_scan_rounds", max_scan_rounds)
                .field(
                    "match_pattern_bytes",
                    &match_pattern.as_ref().map(|p| p.len()),
                ),
            Self::RedisCollectionScan { key, kind, options } => debug
                .field("key_bytes", &key.len())
                .field("kind", kind)
                .field("options", options),
            Self::RedisBlockingPop {
                key,
                limits,
                max_cell_bytes,
            } => debug
                .field("key_bytes", &key.len())
                .field("limits", limits)
                .field("max_cell_bytes", max_cell_bytes),
            Self::RedisSubscribe {
                selector,
                kind,
                options,
            } => debug
                .field("selector_bytes", &selector.len())
                .field("kind", kind)
                .field("options", options),
        };
        debug.finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterFailureClass {
    EngineMismatch,
    InvalidRequest,
    Query,
    Connection,
    Timeout,
    Authentication,
    Protocol,
    Decode,
    ResourceLimit,
    Page,
    CancellationTransport,
    ClientCancelled,
    ServerCancelled,
    WriteOutcomeUnknown,
    /// Insufficient privilege (e.g. pg_cancel_backend without rights).
    PermissionDenied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdapterError {
    engine: Engine,
    class: AdapterFailureClass,
}

impl AdapterError {
    #[must_use]
    pub const fn new(engine: Engine, class: AdapterFailureClass) -> Self {
        Self { engine, class }
    }

    #[must_use]
    pub const fn engine(self) -> Engine {
        self.engine
    }

    #[must_use]
    pub const fn class(self) -> AdapterFailureClass {
        self.class
    }
}

impl fmt::Display for AdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.class == AdapterFailureClass::PermissionDenied {
            return write!(formatter, "permission denied ({:?})", self.engine);
        }
        write!(
            formatter,
            "{:?} adapter operation failed ({:?})",
            self.engine, self.class
        )
    }
}

impl Error for AdapterError {}

pub trait DriverPageStream: Send {
    fn next_page<'a>(
        &'a mut self,
        identity: PageIdentity,
        start_row: u64,
    ) -> DriverFuture<'a, Result<Option<ResultPage>, AdapterError>>;

    /// Optional server progress label (e.g. ClickHouse X-ClickHouse-Summary).
    /// Default: none. Safe to call after stream start / page reads.
    fn progress_label(&self) -> Option<String> {
        None
    }
}

pub trait DriverSession: Send + Sync {
    fn engine(&self) -> Engine;

    fn start_page_stream<'a>(
        &'a self,
        request: DriverPageRequest,
    ) -> DriverFuture<'a, Result<Box<dyn DriverPageStream>, AdapterError>>;

    fn cancel<'a>(&'a self, operation_id: OperationId) -> DriverFuture<'a, CancelDispatch>;

    /// Cheap round-trip proving the session can still reach the server.
    fn health<'a>(&'a self) -> DriverFuture<'a, Result<SessionHealth, AdapterError>>;

    /// Lazy catalog level listing for the workbench sidebar.
    fn catalog<'a>(
        &'a self,
        request: CatalogRequest,
    ) -> DriverFuture<'a, Result<CatalogSubtree, AdapterError>>;

    /// Bounded server identity/version facts for Test Connection.
    fn describe<'a>(&'a self) -> DriverFuture<'a, Result<ServerDescribe, AdapterError>>;

    /// Non-blocking drain of server notices (PostgreSQL). Default empty.
    /// Lines are severity + message only; never include SQL or values.
    fn drain_server_notices<'a>(&'a self) -> DriverFuture<'a, Vec<String>> {
        Box::pin(async { Vec::new() })
    }

    /// Apply an authorized mutation plan (real engines override; stubs fail closed).
    fn apply_authorized_mutation<'a>(
        &'a self,
        authorized: AuthorizedMutationPlan,
    ) -> DriverFuture<'a, Result<MutationApplyOutcome, AdapterError>> {
        let _ = authorized;
        Box::pin(async {
            Err(AdapterError::new(
                self.engine(),
                AdapterFailureClass::InvalidRequest,
            ))
        })
    }

    /// Execute a reviewed typed DDL plan (identifiers quoted at the engine).
    fn execute_ddl_plan<'a>(
        &'a self,
        plan: tablerock_core::DdlPlan,
    ) -> DriverFuture<'a, Result<(), AdapterError>> {
        let _ = plan;
        Box::pin(async {
            Err(AdapterError::new(
                self.engine(),
                AdapterFailureClass::InvalidRequest,
            ))
        })
    }

    /// PostgreSQL-only: role list + effective membership + optional table grants.
    ///
    /// Lines are presentation-ready for the inspector panel. Non-PG engines
    /// return [`AdapterFailureClass::EngineMismatch`].
    fn role_inspector_lines<'a>(
        &'a self,
        schema: Option<&'a str>,
        table: Option<&'a str>,
    ) -> DriverFuture<'a, Result<Vec<String>, AdapterError>> {
        let _ = (schema, table);
        Box::pin(async {
            Err(AdapterError::new(
                self.engine(),
                AdapterFailureClass::EngineMismatch,
            ))
        })
    }

    /// Execute one operator-authorized startup statement (Write/Dangerous after review).
    ///
    /// Callers must have completed the review gate; this does not re-check safety class.
    fn execute_startup_authorized<'a>(
        &'a self,
        statement: &'a str,
        timeout_ms: u32,
    ) -> DriverFuture<'a, Result<(), AdapterError>> {
        let _ = (statement, timeout_ms);
        Box::pin(async {
            Err(AdapterError::new(
                self.engine(),
                AdapterFailureClass::InvalidRequest,
            ))
        })
    }

    /// ClickHouse-only: kill one unfinished async mutation by id.
    ///
    /// Parameters are bound; empty or non-id mutation tokens fail closed.
    /// Non-ClickHouse engines return [`AdapterFailureClass::EngineMismatch`].
    fn kill_clickhouse_mutation<'a>(
        &'a self,
        database: &'a str,
        table: &'a str,
        mutation_id: &'a str,
    ) -> DriverFuture<'a, Result<(), AdapterError>> {
        let _ = (database, table, mutation_id);
        Box::pin(async {
            Err(AdapterError::new(
                self.engine(),
                AdapterFailureClass::EngineMismatch,
            ))
        })
    }

    /// Redis-only: load a type-specific key view as display lines.
    ///
    /// `collection_skip` skips entries for hash/set/zset pages (0 = first page).
    /// Returns `next_skip` when more collection entries remain.
    fn redis_key_view_lines<'a>(
        &'a self,
        key: &'a [u8],
        collection_skip: u64,
    ) -> DriverFuture<'a, Result<(String, Vec<String>, Option<u64>), AdapterError>> {
        let _ = (key, collection_skip);
        Box::pin(async {
            Err(AdapterError::new(
                self.engine(),
                AdapterFailureClass::EngineMismatch,
            ))
        })
    }

    /// Redis-only: bounded INFO snapshot lines + sample time.
    fn redis_info_lines<'a>(
        &'a self,
    ) -> DriverFuture<'a, Result<(u64, Vec<String>), AdapterError>> {
        Box::pin(async {
            Err(AdapterError::new(
                self.engine(),
                AdapterFailureClass::EngineMismatch,
            ))
        })
    }

    /// Redis-only: sequential non-transactional command pipeline outcomes.
    fn redis_execute_pipeline<'a>(
        &'a self,
        commands: &'a [crate::RedisPipelineCommand],
    ) -> DriverFuture<'a, Result<Vec<crate::RedisPipelineOutcome>, AdapterError>> {
        let _ = commands;
        Box::pin(async {
            Err(AdapterError::new(
                self.engine(),
                AdapterFailureClass::EngineMismatch,
            ))
        })
    }

    fn shutdown(self: Box<Self>) -> DriverFuture<'static, Result<(), AdapterError>>;
}

pub(crate) fn measure_health<'a, F, Fut>(
    engine: Engine,
    work: F,
) -> DriverFuture<'a, Result<SessionHealth, AdapterError>>
where
    F: FnOnce() -> Fut + Send + 'a,
    Fut: Future<Output = Result<(), AdapterError>> + Send + 'a,
{
    Box::pin(async move {
        let started = Instant::now();
        work().await?;
        Ok(SessionHealth::new(
            engine,
            true,
            u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        ))
    })
}

impl DriverPageStream for PostgresRowStream {
    fn next_page<'a>(
        &'a mut self,
        identity: PageIdentity,
        start_row: u64,
    ) -> DriverFuture<'a, Result<Option<ResultPage>, AdapterError>> {
        Box::pin(async move {
            PostgresRowStream::next_page(self, identity, start_row)
                .await
                .map_err(map_postgres)
        })
    }
}

impl DriverPageStream for ClickHouseRowStream {
    fn next_page<'a>(
        &'a mut self,
        identity: PageIdentity,
        start_row: u64,
    ) -> DriverFuture<'a, Result<Option<ResultPage>, AdapterError>> {
        Box::pin(async move {
            ClickHouseRowStream::next_page(self, identity, start_row)
                .await
                .map_err(map_clickhouse)
        })
    }

    fn progress_label(&self) -> Option<String> {
        ClickHouseRowStream::progress_label(self).map(str::to_owned)
    }
}

impl DriverPageStream for RedisKeyStream {
    fn next_page<'a>(
        &'a mut self,
        identity: PageIdentity,
        start_row: u64,
    ) -> DriverFuture<'a, Result<Option<ResultPage>, AdapterError>> {
        Box::pin(async move {
            RedisKeyStream::next_page(self, identity, start_row)
                .await
                .map_err(map_redis)
        })
    }
}

impl DriverPageStream for RedisCollectionStream {
    fn next_page<'a>(
        &'a mut self,
        identity: PageIdentity,
        start_row: u64,
    ) -> DriverFuture<'a, Result<Option<ResultPage>, AdapterError>> {
        Box::pin(async move {
            RedisCollectionStream::next_page(self, identity, start_row)
                .await
                .map_err(map_redis)
        })
    }
}

impl DriverPageStream for crate::RedisBlockingPopStream {
    fn next_page<'a>(
        &'a mut self,
        identity: PageIdentity,
        start_row: u64,
    ) -> DriverFuture<'a, Result<Option<ResultPage>, AdapterError>> {
        Box::pin(async move {
            crate::RedisBlockingPopStream::next_page(self, identity, start_row)
                .await
                .map_err(map_redis)
        })
    }
}

impl DriverPageStream for RedisSubscriptionStream {
    fn next_page<'a>(
        &'a mut self,
        identity: PageIdentity,
        start_row: u64,
    ) -> DriverFuture<'a, Result<Option<ResultPage>, AdapterError>> {
        Box::pin(async move {
            RedisSubscriptionStream::next_page(self, identity, start_row)
                .await
                .map_err(map_redis)
        })
    }
}

impl DriverSession for PostgresSession {
    fn engine(&self) -> Engine {
        Engine::PostgreSql
    }

    fn drain_server_notices<'a>(&'a self) -> DriverFuture<'a, Vec<String>> {
        Box::pin(async move {
            let deliveries = self.try_drain_notices(8).await;
            deliveries
                .into_iter()
                .map(|delivery| match delivery {
                    crate::PostgresNoticeDelivery::Notice(notice) => {
                        let mut line = format!("{}: {}", notice.severity(), notice.message());
                        if let Some(detail) = notice.detail() {
                            if !detail.is_empty() {
                                line.push_str(" · detail: ");
                                line.push_str(detail);
                            }
                        }
                        if let Some(hint) = notice.hint() {
                            if !hint.is_empty() {
                                line.push_str(" · hint: ");
                                line.push_str(hint);
                            }
                        }
                        line
                    }
                    crate::PostgresNoticeDelivery::Overflow { dropped } => {
                        format!("NOTICE overflow: dropped {dropped}")
                    }
                })
                .collect()
        })
    }

    fn start_page_stream<'a>(
        &'a self,
        request: DriverPageRequest,
    ) -> DriverFuture<'a, Result<Box<dyn DriverPageStream>, AdapterError>> {
        Box::pin(async move {
            match request {
                DriverPageRequest::PostgreSqlProbe {
                    query,
                    limits,
                    max_cell_bytes,
                } => self
                    .stream_probe(query, limits, max_cell_bytes)
                    .await
                    .map(|stream| Box::new(stream) as Box<dyn DriverPageStream>)
                    .map_err(map_postgres),
                DriverPageRequest::PostgreSqlStatement {
                    statement,
                    parameters,
                    limits,
                    max_cell_bytes,
                } => self
                    .stream_statement(statement.as_str(), &parameters, limits, max_cell_bytes)
                    .await
                    .map(|stream| Box::new(stream) as Box<dyn DriverPageStream>)
                    .map_err(map_postgres),
                _ => Err(AdapterError::new(
                    Engine::PostgreSql,
                    AdapterFailureClass::EngineMismatch,
                )),
            }
        })
    }

    fn cancel<'a>(&'a self, _operation_id: OperationId) -> DriverFuture<'a, CancelDispatch> {
        Box::pin(async {
            match self.dispatch_cancel().await {
                Ok(()) => CancelDispatch::RequestSent,
                Err(_) => CancelDispatch::TransportFailed,
            }
        })
    }

    fn health<'a>(&'a self) -> DriverFuture<'a, Result<SessionHealth, AdapterError>> {
        measure_health(Engine::PostgreSql, || async {
            self.health_check().await.map_err(map_postgres)
        })
    }

    fn catalog<'a>(
        &'a self,
        request: CatalogRequest,
    ) -> DriverFuture<'a, Result<CatalogSubtree, AdapterError>> {
        Box::pin(async move { self.list_catalog(request).await.map_err(map_postgres) })
    }

    fn describe<'a>(&'a self) -> DriverFuture<'a, Result<ServerDescribe, AdapterError>> {
        Box::pin(async move { self.describe_server().await.map_err(map_postgres) })
    }

    fn apply_authorized_mutation<'a>(
        &'a self,
        authorized: AuthorizedMutationPlan,
    ) -> DriverFuture<'a, Result<MutationApplyOutcome, AdapterError>> {
        Box::pin(async move {
            PostgresSession::apply_authorized_mutation(self, authorized)
                .await
                .map_err(map_postgres)
        })
    }

    fn execute_ddl_plan<'a>(
        &'a self,
        plan: tablerock_core::DdlPlan,
    ) -> DriverFuture<'a, Result<(), AdapterError>> {
        Box::pin(async move {
            PostgresSession::execute_ddl_plan(self, &plan)
                .await
                .map_err(map_postgres)
        })
    }

    fn role_inspector_lines<'a>(
        &'a self,
        schema: Option<&'a str>,
        table: Option<&'a str>,
    ) -> DriverFuture<'a, Result<Vec<String>, AdapterError>> {
        Box::pin(async move {
            PostgresSession::role_inspector_lines(self, schema, table)
                .await
                .map_err(map_postgres)
        })
    }

    fn execute_startup_authorized<'a>(
        &'a self,
        statement: &'a str,
        timeout_ms: u32,
    ) -> DriverFuture<'a, Result<(), AdapterError>> {
        Box::pin(async move {
            let timeout = std::time::Duration::from_millis(u64::from(timeout_ms.max(100)));
            match tokio::time::timeout(timeout, self.execute_sql(statement)).await {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(map_postgres(e)),
                Err(_) => Err(AdapterError::new(
                    Engine::PostgreSql,
                    AdapterFailureClass::Timeout,
                )),
            }
        })
    }

    fn shutdown(self: Box<Self>) -> DriverFuture<'static, Result<(), AdapterError>> {
        Box::pin(async move { (*self).shutdown().await.map_err(map_postgres) })
    }
}

impl DriverSession for ClickHouseSession {
    fn engine(&self) -> Engine {
        Engine::ClickHouse
    }

    fn start_page_stream<'a>(
        &'a self,
        request: DriverPageRequest,
    ) -> DriverFuture<'a, Result<Box<dyn DriverPageStream>, AdapterError>> {
        Box::pin(async move {
            match request {
                DriverPageRequest::ClickHouseProbe {
                    query,
                    query_id,
                    limits,
                    max_cell_bytes,
                } => self
                    .stream_probe(query, &query_id, limits, max_cell_bytes)
                    .await
                    .map(|stream| Box::new(stream) as Box<dyn DriverPageStream>)
                    .map_err(map_clickhouse),
                DriverPageRequest::ClickHouseStatement {
                    statement,
                    query_id,
                    limits,
                    max_cell_bytes,
                } => self
                    .stream_statement(statement.as_str(), &query_id, limits, max_cell_bytes)
                    .await
                    .map(|stream| Box::new(stream) as Box<dyn DriverPageStream>)
                    .map_err(map_clickhouse),
                _ => Err(AdapterError::new(
                    Engine::ClickHouse,
                    AdapterFailureClass::EngineMismatch,
                )),
            }
        })
    }

    fn cancel<'a>(&'a self, _operation_id: OperationId) -> DriverFuture<'a, CancelDispatch> {
        Box::pin(async {
            match self.dispatch_cancel().await {
                Ok(true) => CancelDispatch::RequestSent,
                Ok(false) => CancelDispatch::ServerRejected,
                Err(_) => CancelDispatch::TransportFailed,
            }
        })
    }

    fn health<'a>(&'a self) -> DriverFuture<'a, Result<SessionHealth, AdapterError>> {
        measure_health(Engine::ClickHouse, || async {
            self.health_check().await.map_err(map_clickhouse)
        })
    }

    fn catalog<'a>(
        &'a self,
        request: CatalogRequest,
    ) -> DriverFuture<'a, Result<CatalogSubtree, AdapterError>> {
        Box::pin(async move { self.list_catalog(request).await.map_err(map_clickhouse) })
    }

    fn describe<'a>(&'a self) -> DriverFuture<'a, Result<ServerDescribe, AdapterError>> {
        Box::pin(async move { self.describe_server().await.map_err(map_clickhouse) })
    }

    fn apply_authorized_mutation<'a>(
        &'a self,
        authorized: AuthorizedMutationPlan,
    ) -> DriverFuture<'a, Result<MutationApplyOutcome, AdapterError>> {
        Box::pin(async move {
            ClickHouseSession::apply_authorized_mutation(self, authorized)
                .await
                .map_err(map_clickhouse)
        })
    }

    fn kill_clickhouse_mutation<'a>(
        &'a self,
        database: &'a str,
        table: &'a str,
        mutation_id: &'a str,
    ) -> DriverFuture<'a, Result<(), AdapterError>> {
        Box::pin(async move {
            self.kill_mutation(database, table, mutation_id)
                .await
                .map_err(map_clickhouse)
        })
    }

    fn execute_startup_authorized<'a>(
        &'a self,
        statement: &'a str,
        timeout_ms: u32,
    ) -> DriverFuture<'a, Result<(), AdapterError>> {
        Box::pin(async move {
            let timeout = std::time::Duration::from_millis(u64::from(timeout_ms.max(100)));
            match tokio::time::timeout(timeout, self.execute_sql(statement)).await {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(map_clickhouse(e)),
                Err(_) => Err(AdapterError::new(
                    Engine::ClickHouse,
                    AdapterFailureClass::Timeout,
                )),
            }
        })
    }

    fn shutdown(self: Box<Self>) -> DriverFuture<'static, Result<(), AdapterError>> {
        Box::pin(async move {
            drop(self);
            Ok(())
        })
    }
}

impl DriverSession for RedisSession {
    fn engine(&self) -> Engine {
        Engine::Redis
    }

    fn start_page_stream<'a>(
        &'a self,
        request: DriverPageRequest,
    ) -> DriverFuture<'a, Result<Box<dyn DriverPageStream>, AdapterError>> {
        Box::pin(async move {
            match request {
                DriverPageRequest::RedisKeyScan {
                    limits,
                    max_cell_bytes,
                    scan_count,
                    max_scan_rounds,
                    match_pattern,
                } => self
                    .scan_keys(
                        limits,
                        max_cell_bytes,
                        scan_count,
                        max_scan_rounds,
                        match_pattern,
                    )
                    .map(|stream| Box::new(stream) as Box<dyn DriverPageStream>)
                    .map_err(map_redis),
                DriverPageRequest::RedisCollectionScan { key, kind, options } => self
                    .scan_collection(key, kind, options)
                    .map(|stream| Box::new(stream) as Box<dyn DriverPageStream>)
                    .map_err(map_redis),
                DriverPageRequest::RedisBlockingPop {
                    key,
                    limits,
                    max_cell_bytes,
                } => self
                    .blocking_pop(key, limits, max_cell_bytes)
                    .await
                    .map(|stream| Box::new(stream) as Box<dyn DriverPageStream>)
                    .map_err(map_redis),
                DriverPageRequest::RedisSubscribe {
                    selector,
                    kind,
                    options,
                } => match kind {
                    RedisSubscriptionKind::Channel => self.subscribe(selector, options).await,
                    RedisSubscriptionKind::Pattern => self.psubscribe(selector, options).await,
                }
                .map(|stream| Box::new(stream) as Box<dyn DriverPageStream>)
                .map_err(map_redis),
                _ => Err(AdapterError::new(
                    Engine::Redis,
                    AdapterFailureClass::EngineMismatch,
                )),
            }
        })
    }

    fn cancel<'a>(&'a self, _operation_id: OperationId) -> DriverFuture<'a, CancelDispatch> {
        Box::pin(async {
            match self.dispatch_cancel().await {
                Ok(crate::RedisCancelDispatch::PreventedBeforeDispatch) => {
                    CancelDispatch::PreventedBeforeDispatch
                }
                Ok(crate::RedisCancelDispatch::RequestSent) => CancelDispatch::RequestSent,
                Ok(crate::RedisCancelDispatch::ServerRejected) => CancelDispatch::ServerRejected,
                Err(_) => CancelDispatch::TransportFailed,
            }
        })
    }

    fn health<'a>(&'a self) -> DriverFuture<'a, Result<SessionHealth, AdapterError>> {
        measure_health(Engine::Redis, || async {
            self.health_check().await.map_err(map_redis)
        })
    }

    fn catalog<'a>(
        &'a self,
        request: CatalogRequest,
    ) -> DriverFuture<'a, Result<CatalogSubtree, AdapterError>> {
        Box::pin(async move { self.list_catalog(request).await.map_err(map_redis) })
    }

    fn describe<'a>(&'a self) -> DriverFuture<'a, Result<ServerDescribe, AdapterError>> {
        Box::pin(async move { self.describe_server().await.map_err(map_redis) })
    }

    fn apply_authorized_mutation<'a>(
        &'a self,
        authorized: AuthorizedMutationPlan,
    ) -> DriverFuture<'a, Result<MutationApplyOutcome, AdapterError>> {
        Box::pin(async move {
            RedisSession::apply_authorized_mutation(self, authorized)
                .await
                .map_err(map_redis)
        })
    }

    fn redis_key_view_lines<'a>(
        &'a self,
        key: &'a [u8],
        collection_skip: u64,
    ) -> DriverFuture<'a, Result<(String, Vec<String>, Option<u64>), AdapterError>> {
        Box::pin(async move {
            use tablerock_core::{BoundedBytes, ByteLimit, RedisKeyKind};
            let key = BoundedBytes::copy_from_slice(key, ByteLimit::new(key.len() as u64 + 1))
                .map_err(|_| {
                    AdapterError::new(Engine::Redis, AdapterFailureClass::InvalidRequest)
                })?;
            let kind = self.key_type(&key).await.map_err(map_redis)?;
            let ttl = self
                .read_time_to_live(&key)
                .await
                .map(|t| format!("{t:?}"))
                .unwrap_or_else(|_| "unavailable".into());
            let mut lines = vec![format!("type: {kind:?}"), format!("ttl: {ttl}")];
            let kind_label = match kind {
                RedisKeyKind::String => "string",
                RedisKeyKind::Hash => "hash",
                RedisKeyKind::List => "list",
                RedisKeyKind::Set => "set",
                RedisKeyKind::SortedSet => "zset",
                RedisKeyKind::Stream => "stream",
                RedisKeyKind::Unknown => "unknown",
            };
            let mut next_skip = None;
            match kind {
                RedisKeyKind::String => {
                    if let Ok(Some(v)) = self.read_binary(&key, 4 * 1024).await {
                        let bytes = match v.as_ref() {
                            tablerock_core::ValueRef::Binary { value, .. } => value.to_vec(),
                            tablerock_core::ValueRef::Text { value, .. } => {
                                value.as_bytes().to_vec()
                            }
                            _ => Vec::new(),
                        };
                        lines.push(format!("value: {}", String::from_utf8_lossy(&bytes)));
                        if v.is_truncated() {
                            lines.push("truncated: yes".into());
                        }
                    } else {
                        lines.push("value: (missing or empty)".into());
                    }
                }
                RedisKeyKind::List => {
                    if let Ok(vals) = self.list_range(&key, 0, 31, 32, 256).await {
                        for (i, v) in vals.iter().enumerate() {
                            let text = match v.as_ref() {
                                tablerock_core::ValueRef::Binary { value, .. } => {
                                    String::from_utf8_lossy(value).into_owned()
                                }
                                tablerock_core::ValueRef::Text { value, .. } => value.to_string(),
                                _ => format!("{v:?}"),
                            };
                            lines.push(format!("{i}: {text}"));
                        }
                    }
                }
                RedisKeyKind::Stream => {
                    if let Ok(entries) = self.stream_range(&key, "-", "+", 16, 256).await {
                        for e in entries {
                            lines.push(format!("{} {}", e.id, e.fields.join("=")));
                        }
                    }
                }
                RedisKeyKind::Hash | RedisKeyKind::Set | RedisKeyKind::SortedSet => {
                    match redis_collection_page_lines(self, &key, kind, collection_skip).await {
                        Ok((page_lines, next)) => {
                            lines.extend(page_lines);
                            next_skip = next;
                        }
                        Err(error) => lines.push(format!("collection scan failed: {error}")),
                    }
                }
                RedisKeyKind::Unknown => {
                    lines.push("key missing or type unknown".into());
                }
            }
            Ok((kind_label.into(), lines, next_skip))
        })
    }

    fn redis_info_lines<'a>(
        &'a self,
    ) -> DriverFuture<'a, Result<(u64, Vec<String>), AdapterError>> {
        Box::pin(async move {
            let snap = self.server_info_snapshot().await.map_err(map_redis)?;
            let lines: Vec<String> = snap
                .fields
                .into_iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect();
            Ok((snap.sampled_at_ms, lines))
        })
    }

    fn redis_execute_pipeline<'a>(
        &'a self,
        commands: &'a [crate::RedisPipelineCommand],
    ) -> DriverFuture<'a, Result<Vec<crate::RedisPipelineOutcome>, AdapterError>> {
        Box::pin(async move { self.execute_pipeline(commands).await.map_err(map_redis) })
    }

    fn execute_startup_authorized<'a>(
        &'a self,
        statement: &'a str,
        timeout_ms: u32,
    ) -> DriverFuture<'a, Result<(), AdapterError>> {
        Box::pin(async move {
            let mut parts = statement.split_whitespace();
            let Some(name) = parts.next() else {
                return Err(AdapterError::new(
                    Engine::Redis,
                    AdapterFailureClass::InvalidRequest,
                ));
            };
            let args: Vec<Vec<u8>> = parts.map(|p| p.as_bytes().to_vec()).collect();
            let timeout = std::time::Duration::from_millis(u64::from(timeout_ms.max(100)));
            match tokio::time::timeout(
                timeout,
                self.execute_command_argv(&name.to_ascii_uppercase(), &args),
            )
            .await
            {
                Ok(Ok(_)) => Ok(()),
                Ok(Err(e)) => Err(map_redis(e)),
                Err(_) => Err(AdapterError::new(
                    Engine::Redis,
                    AdapterFailureClass::Timeout,
                )),
            }
        })
    }

    fn shutdown(self: Box<Self>) -> DriverFuture<'static, Result<(), AdapterError>> {
        Box::pin(async move {
            drop(self);
            Ok(())
        })
    }
}

fn map_postgres(error: PostgresError) -> AdapterError {
    let class = match error {
        PostgresError::Connect => AdapterFailureClass::Connection,
        PostgresError::Query => AdapterFailureClass::Query,
        PostgresError::Connection => AdapterFailureClass::Connection,
        PostgresError::Protocol => AdapterFailureClass::Protocol,
        PostgresError::CancellationTransport => AdapterFailureClass::CancellationTransport,
        PostgresError::TlsConfiguration => AdapterFailureClass::InvalidRequest,
        PostgresError::ServerCancelled => AdapterFailureClass::ServerCancelled,
        PostgresError::InvalidLimits => AdapterFailureClass::InvalidRequest,
        PostgresError::CopyLimitExceeded => AdapterFailureClass::InvalidRequest,
        PostgresError::WriteOutcomeUnknown => AdapterFailureClass::WriteOutcomeUnknown,
        PostgresError::PermissionDenied => AdapterFailureClass::PermissionDenied,
        PostgresError::Page(_) => AdapterFailureClass::Page,
    };
    AdapterError::new(Engine::PostgreSql, class)
}

fn map_clickhouse(error: ClickHouseError) -> AdapterError {
    let class = match error {
        ClickHouseError::Query => AdapterFailureClass::Query,
        ClickHouseError::Protocol => AdapterFailureClass::Protocol,
        ClickHouseError::UnsupportedType => AdapterFailureClass::Decode,
        ClickHouseError::ServerCancelled => AdapterFailureClass::ServerCancelled,
        ClickHouseError::SessionBusy => AdapterFailureClass::InvalidRequest,
        ClickHouseError::InvalidLimits => AdapterFailureClass::InvalidRequest,
        ClickHouseError::Page(_) => AdapterFailureClass::Page,
    };
    AdapterError::new(Engine::ClickHouse, class)
}

/// Bounded collection page with skip/take. Rescans from 0 and skips entries
/// so next-page does not require holding stream state across effects.
async fn redis_collection_page_lines(
    session: &RedisSession,
    key: &tablerock_core::BoundedBytes,
    kind: tablerock_core::RedisKeyKind,
    skip: u64,
) -> Result<(Vec<String>, Option<u64>), AdapterError> {
    use crate::{RedisCollectionScanKind, RedisCollectionScanOptions};
    use tablerock_core::{IdParts, PageIdentity, PageLimits, ResultId, Revision, ValueKind};

    const TAKE: u32 = 32;
    let scan_kind = match kind {
        tablerock_core::RedisKeyKind::Hash => RedisCollectionScanKind::Hash,
        tablerock_core::RedisKeyKind::Set => RedisCollectionScanKind::Set,
        tablerock_core::RedisKeyKind::SortedSet => RedisCollectionScanKind::SortedSet,
        _ => {
            return Err(AdapterError::new(
                Engine::Redis,
                AdapterFailureClass::InvalidRequest,
            ));
        }
    };
    let cmd = match scan_kind {
        RedisCollectionScanKind::Hash => "HSCAN",
        RedisCollectionScanKind::Set => "SSCAN",
        RedisCollectionScanKind::SortedSet => "ZSCAN",
    };
    // Larger round budget so skip+take can rescan from the start.
    let options = RedisCollectionScanOptions::new(
        PageLimits::new(TAKE, 2, 64 * 1024, 256),
        256,
        32,
        128,
        128 * 1024,
        256,
    );
    let mut stream = session
        .scan_collection(key.clone(), scan_kind, options)
        .map_err(map_redis)?;
    let mut lines = Vec::new();
    let mut skipped = 0_u64;
    let mut taken = 0_u32;
    let mut start_row = 0_u64;
    let mut has_more = false;
    let mut page_no = 0_u32;
    loop {
        let identity = PageIdentity::new(
            ResultId::from_parts(IdParts::new(1, 9_100 + u64::from(page_no)).unwrap()).unwrap(),
            Revision::INITIAL,
            Engine::Redis,
        );
        page_no = page_no.saturating_add(1);
        match stream.next_page(identity, start_row).await {
            Ok(Some(page)) => {
                let rows = page.envelope().row_count();
                for row in 0..rows {
                    if skipped < skip {
                        skipped += 1;
                        continue;
                    }
                    if taken >= TAKE {
                        has_more = true;
                        break;
                    }
                    let a = page
                        .cell(row, 0)
                        .map(|c| String::from_utf8_lossy(c.bytes()).into_owned())
                        .unwrap_or_default();
                    match scan_kind {
                        RedisCollectionScanKind::Set => lines.push(format!("  {a}")),
                        RedisCollectionScanKind::Hash | RedisCollectionScanKind::SortedSet => {
                            let b = page
                                .cell(row, 1)
                                .map(|c| {
                                    if c.kind() == ValueKind::Float64 {
                                        let mut buf = [0u8; 8];
                                        let bytes = c.bytes();
                                        let n = bytes.len().min(8);
                                        buf[8 - n..].copy_from_slice(&bytes[..n]);
                                        f64::from_bits(u64::from_be_bytes(buf)).to_string()
                                    } else {
                                        String::from_utf8_lossy(c.bytes()).into_owned()
                                    }
                                })
                                .unwrap_or_default();
                            lines.push(format!("  {a} = {b}"));
                        }
                    }
                    taken += 1;
                }
                start_row = start_row.saturating_add(u64::from(rows));
                if has_more {
                    break;
                }
                // Continue until stream complete when still filling skip/take.
            }
            Ok(None) => break,
            Err(error) => return Err(map_redis(error)),
        }
    }
    let mut header = vec![format!(
        "{cmd} page skip={skip} take={taken}{}",
        if has_more { " (more)" } else { " (end)" }
    )];
    if lines.is_empty() && skip == 0 {
        header.push(format!("{cmd}: empty"));
    }
    header.extend(lines);
    if has_more {
        header.push("  … more (RMore for next page)".into());
    }
    let next = if has_more {
        Some(skip.saturating_add(u64::from(taken)))
    } else {
        None
    };
    Ok((header, next))
}

fn map_redis(error: RedisError) -> AdapterError {
    let class = match error {
        RedisError::Connect => AdapterFailureClass::Connection,
        RedisError::Connection => AdapterFailureClass::Connection,
        RedisError::Timeout => AdapterFailureClass::Timeout,
        RedisError::Authentication => AdapterFailureClass::Authentication,
        RedisError::TlsConfiguration => AdapterFailureClass::InvalidRequest,
        RedisError::Command => AdapterFailureClass::Query,
        RedisError::ClientCancelled => AdapterFailureClass::ClientCancelled,
        RedisError::ServerCancelled => AdapterFailureClass::ServerCancelled,
        RedisError::SessionBusy => AdapterFailureClass::InvalidRequest,
        RedisError::InvalidLimits => AdapterFailureClass::InvalidRequest,
        RedisError::ScanBudgetExhausted => AdapterFailureClass::Query,
        RedisError::ScanResponseLimitExceeded => AdapterFailureClass::ResourceLimit,
        RedisError::SubscriptionOverflow => AdapterFailureClass::ResourceLimit,
        RedisError::InvalidMutation | RedisError::LogicalDatabaseMismatch => {
            AdapterFailureClass::InvalidRequest
        }
        RedisError::WriteOutcomeUnknown => AdapterFailureClass::WriteOutcomeUnknown,
        RedisError::Protocol => AdapterFailureClass::Protocol,
        RedisError::Page(_) => AdapterFailureClass::Page,
    };
    AdapterError::new(Engine::Redis, class)
}

#[cfg(test)]
mod redis_mapping_tests {
    use super::*;

    #[test]
    fn redis_transport_failures_keep_their_stable_adapter_classes() {
        assert_eq!(
            map_redis(RedisError::Timeout).class(),
            AdapterFailureClass::Timeout
        );
        assert_eq!(
            map_redis(RedisError::Connection).class(),
            AdapterFailureClass::Connection
        );
        assert_eq!(
            map_redis(RedisError::Authentication).class(),
            AdapterFailureClass::Authentication
        );
        assert_eq!(
            map_redis(RedisError::TlsConfiguration).class(),
            AdapterFailureClass::InvalidRequest
        );
        assert_eq!(
            map_redis(RedisError::WriteOutcomeUnknown).class(),
            AdapterFailureClass::WriteOutcomeUnknown
        );
        assert_eq!(
            map_redis(RedisError::LogicalDatabaseMismatch).class(),
            AdapterFailureClass::InvalidRequest
        );
    }
}
