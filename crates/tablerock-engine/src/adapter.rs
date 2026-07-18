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
            } => debug
                .field("limits", limits)
                .field("max_cell_bytes", max_cell_bytes)
                .field("scan_count", scan_count)
                .field("max_scan_rounds", max_scan_rounds),
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

    /// Redis-only: load a type-specific key view as display lines.
    fn redis_key_view_lines<'a>(
        &'a self,
        key: &'a [u8],
    ) -> DriverFuture<'a, Result<(String, Vec<String>), AdapterError>> {
        let _ = key;
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
                } => self
                    .scan_keys(limits, max_cell_bytes, scan_count, max_scan_rounds)
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
    ) -> DriverFuture<'a, Result<(String, Vec<String>), AdapterError>> {
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
            let mut lines = vec![
                format!("type: {kind:?}"),
                format!("ttl: {ttl}"),
            ];
            let kind_label = match kind {
                RedisKeyKind::String => "string",
                RedisKeyKind::Hash => "hash",
                RedisKeyKind::List => "list",
                RedisKeyKind::Set => "set",
                RedisKeyKind::SortedSet => "zset",
                RedisKeyKind::Stream => "stream",
                RedisKeyKind::Unknown => "unknown",
            };
            match kind {
                RedisKeyKind::String => {
                    if let Ok(Some(v)) = self.read_binary(&key, 4 * 1024).await {
                        let bytes = match v.as_ref() {
                            tablerock_core::ValueRef::Binary { value, .. } => value.to_vec(),
                            tablerock_core::ValueRef::Text { value, .. } => value.as_bytes().to_vec(),
                            _ => Vec::new(),
                        };
                        lines.push(format!(
                            "value: {}",
                            String::from_utf8_lossy(&bytes)
                        ));
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
                    lines.push("collection: open via HSCAN/SSCAN/ZSCAN (scan_collection)".into());
                }
                RedisKeyKind::Unknown => {
                    lines.push("key missing or type unknown".into());
                }
            }
            Ok((kind_label.into(), lines))
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
