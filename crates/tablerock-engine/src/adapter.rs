use std::{error::Error, fmt, future::Future, pin::Pin};

use tablerock_core::{
    BoundedBytes, BoundedText, CancelDispatch, Engine, OperationId, PageIdentity, PageLimits,
    ResultPage,
};

use crate::{
    ClickHouseError, ClickHouseProbeQuery, ClickHouseRowStream, ClickHouseSession, PostgresError,
    PostgresProbeQuery, PostgresRowStream, PostgresSession, RedisCollectionScanKind,
    RedisCollectionScanOptions, RedisCollectionStream, RedisError, RedisKeyStream, RedisSession,
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
}

impl DriverPageRequest {
    #[must_use]
    pub const fn engine(&self) -> Engine {
        match self {
            Self::PostgreSqlProbe { .. } => Engine::PostgreSql,
            Self::ClickHouseProbe { .. } => Engine::ClickHouse,
            Self::RedisKeyScan { .. }
            | Self::RedisCollectionScan { .. }
            | Self::RedisBlockingPop { .. } => Engine::Redis,
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
    Protocol,
    Decode,
    ResourceLimit,
    Page,
    CancellationTransport,
    ClientCancelled,
    ServerCancelled,
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
        Box::pin(async {
            match self.dispatch_cancel().await {
                Ok(()) => CancelDispatch::RequestSent,
                Err(_) => CancelDispatch::TransportFailed,
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
        let DriverPageRequest::ClickHouseProbe {
            query,
            query_id,
            limits,
            max_cell_bytes,
        } = request
        else {
            return Box::pin(async {
                Err(AdapterError::new(
                    Engine::ClickHouse,
                    AdapterFailureClass::EngineMismatch,
                ))
            });
        };
        Box::pin(async move {
            self.stream_probe(query, &query_id, limits, max_cell_bytes)
                .await
                .map(|stream| Box::new(stream) as Box<dyn DriverPageStream>)
                .map_err(map_clickhouse)
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
        RedisError::Command => AdapterFailureClass::Query,
        RedisError::ClientCancelled => AdapterFailureClass::ClientCancelled,
        RedisError::ServerCancelled => AdapterFailureClass::ServerCancelled,
        RedisError::SessionBusy => AdapterFailureClass::InvalidRequest,
        RedisError::InvalidLimits => AdapterFailureClass::InvalidRequest,
        RedisError::ScanBudgetExhausted => AdapterFailureClass::Query,
        RedisError::ScanResponseLimitExceeded => AdapterFailureClass::ResourceLimit,
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
    }
}
