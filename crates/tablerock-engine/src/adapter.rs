use std::{collections::BTreeMap, error::Error, fmt, future::Future, pin::Pin};

use tablerock_core::{BoundedText, Engine, OperationId, PageIdentity, PageLimits, ResultPage};

use crate::{
    ClickHouseError, ClickHouseProbeQuery, ClickHouseRowStream, ClickHouseSession, PostgresError,
    PostgresProbeQuery, PostgresRowStream, PostgresSession, RedisError, RedisKeyStream,
    RedisSession,
};

pub type DriverFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub enum DriverPageRequest {
    PostgreSqlProbe {
        query: PostgresProbeQuery,
        limits: PageLimits,
        max_cell_bytes: u64,
    },
    ClickHouseProbe {
        query: ClickHouseProbeQuery,
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
}

impl DriverPageRequest {
    #[must_use]
    pub const fn engine(&self) -> Engine {
        match self {
            Self::PostgreSqlProbe { .. } => Engine::PostgreSql,
            Self::ClickHouseProbe { .. } => Engine::ClickHouse,
            Self::RedisKeyScan { .. } => Engine::Redis,
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
    Protocol,
    Decode,
    Page,
    CancellationTransport,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelDispatch {
    Unsupported,
    RequestSent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationRegistrationError {
    CapacityExhausted,
    DuplicateOperation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationCancelOutcome {
    UnknownOperation,
    Unsupported,
    RequestSent,
}

/// Bounded routing table from core operation identities to driver sessions.
///
/// This table owns no lifecycle truth: callers retain an entry until the
/// operation reaches an observed terminal state. A cancellation dispatch is
/// never promoted to a terminal cancellation outcome.
pub struct DriverOperationRegistry {
    max_operations: usize,
    sessions: BTreeMap<OperationId, Box<dyn DriverSession>>,
}

impl DriverOperationRegistry {
    #[must_use]
    pub const fn new(max_operations: usize) -> Self {
        Self {
            max_operations,
            sessions: BTreeMap::new(),
        }
    }

    pub fn register(
        &mut self,
        operation_id: OperationId,
        session: Box<dyn DriverSession>,
    ) -> Result<(), OperationRegistrationError> {
        if self.sessions.contains_key(&operation_id) {
            return Err(OperationRegistrationError::DuplicateOperation);
        }
        if self.sessions.len() >= self.max_operations {
            return Err(OperationRegistrationError::CapacityExhausted);
        }
        self.sessions.insert(operation_id, session);
        Ok(())
    }

    pub async fn cancel(&self, operation_id: OperationId) -> OperationCancelOutcome {
        let Some(session) = self.sessions.get(&operation_id) else {
            return OperationCancelOutcome::UnknownOperation;
        };
        match session.cancel(operation_id).await {
            CancelDispatch::Unsupported => OperationCancelOutcome::Unsupported,
            CancelDispatch::RequestSent => OperationCancelOutcome::RequestSent,
        }
    }

    pub fn remove(&mut self, operation_id: OperationId) -> Option<Box<dyn DriverSession>> {
        self.sessions.remove(&operation_id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }
}

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

    fn shutdown(self: Box<Self>) -> DriverFuture<'static, Result<(), AdapterError>>;
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

impl DriverSession for PostgresSession {
    fn engine(&self) -> Engine {
        Engine::PostgreSql
    }

    fn start_page_stream<'a>(
        &'a self,
        request: DriverPageRequest,
    ) -> DriverFuture<'a, Result<Box<dyn DriverPageStream>, AdapterError>> {
        Box::pin(async move {
            let DriverPageRequest::PostgreSqlProbe {
                query,
                limits,
                max_cell_bytes,
            } = request
            else {
                return Err(AdapterError::new(
                    Engine::PostgreSql,
                    AdapterFailureClass::EngineMismatch,
                ));
            };
            self.stream_probe(query, limits, max_cell_bytes)
                .await
                .map(|stream| Box::new(stream) as Box<dyn DriverPageStream>)
                .map_err(map_postgres)
        })
    }

    fn cancel<'a>(&'a self, _operation_id: OperationId) -> DriverFuture<'a, CancelDispatch> {
        Box::pin(async { CancelDispatch::Unsupported })
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
            let DriverPageRequest::ClickHouseProbe {
                query,
                query_id,
                limits,
                max_cell_bytes,
            } = request
            else {
                return Err(AdapterError::new(
                    Engine::ClickHouse,
                    AdapterFailureClass::EngineMismatch,
                ));
            };
            self.stream_probe(query, &query_id, limits, max_cell_bytes)
                .await
                .map(|stream| Box::new(stream) as Box<dyn DriverPageStream>)
                .map_err(map_clickhouse)
        })
    }

    fn cancel<'a>(&'a self, _operation_id: OperationId) -> DriverFuture<'a, CancelDispatch> {
        Box::pin(async { CancelDispatch::Unsupported })
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
            let DriverPageRequest::RedisKeyScan {
                limits,
                max_cell_bytes,
                scan_count,
                max_scan_rounds,
            } = request
            else {
                return Err(AdapterError::new(
                    Engine::Redis,
                    AdapterFailureClass::EngineMismatch,
                ));
            };
            self.scan_keys(limits, max_cell_bytes, scan_count, max_scan_rounds)
                .map(|stream| Box::new(stream) as Box<dyn DriverPageStream>)
                .map_err(map_redis)
        })
    }

    fn cancel<'a>(&'a self, _operation_id: OperationId) -> DriverFuture<'a, CancelDispatch> {
        Box::pin(async { CancelDispatch::Unsupported })
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
        PostgresError::InvalidLimits => AdapterFailureClass::InvalidRequest,
        PostgresError::Page(_) => AdapterFailureClass::Page,
    };
    AdapterError::new(Engine::PostgreSql, class)
}

fn map_clickhouse(error: ClickHouseError) -> AdapterError {
    let class = match error {
        ClickHouseError::Query => AdapterFailureClass::Query,
        ClickHouseError::Protocol => AdapterFailureClass::Protocol,
        ClickHouseError::UnsupportedType => AdapterFailureClass::Decode,
        ClickHouseError::InvalidLimits => AdapterFailureClass::InvalidRequest,
        ClickHouseError::Page(_) => AdapterFailureClass::Page,
    };
    AdapterError::new(Engine::ClickHouse, class)
}

fn map_redis(error: RedisError) -> AdapterError {
    let class = match error {
        RedisError::Connect => AdapterFailureClass::Connection,
        RedisError::Command => AdapterFailureClass::Query,
        RedisError::InvalidLimits => AdapterFailureClass::InvalidRequest,
        RedisError::ScanBudgetExhausted => AdapterFailureClass::Query,
        RedisError::Protocol => AdapterFailureClass::Protocol,
        RedisError::Page(_) => AdapterFailureClass::Page,
    };
    AdapterError::new(Engine::Redis, class)
}
