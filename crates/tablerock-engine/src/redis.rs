use std::{
    collections::VecDeque,
    error::Error,
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use redis::{
    Client, ConnectionAddr, IntoConnectionInfo, ProtocolVersion, RedisConnectionInfo,
    aio::MultiplexedConnection,
};
use tablerock_core::{
    BoundedBytes, BoundedText, ByteLimit, ColumnMetadata, Engine, EngineType, OwnedValue,
    PageDelivery, PageFacts, PageIdentity, PageLimits, PageValidationError, PageWarning,
    PageWarnings, ResultPage, RowTotal, Truncation,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RedisProtocol {
    Resp2,
    Resp3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RedisTlsMode {
    Disable,
    Require,
}

#[derive(Clone, PartialEq, Eq)]
pub struct RedisConnectConfig {
    host: BoundedText,
    port: u16,
    database: i64,
    protocol: RedisProtocol,
    tls: RedisTlsMode,
}

impl RedisConnectConfig {
    #[must_use]
    pub const fn new(
        host: BoundedText,
        port: u16,
        database: i64,
        protocol: RedisProtocol,
        tls: RedisTlsMode,
    ) -> Self {
        Self {
            host,
            port,
            database,
            protocol,
            tls,
        }
    }
}

impl fmt::Debug for RedisConnectConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RedisConnectConfig")
            .field("host_bytes", &self.host.len())
            .field("port", &self.port)
            .field("database", &self.database)
            .field("protocol", &self.protocol)
            .field("tls", &self.tls)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedisError {
    Connect,
    Command,
    ServerCancelled,
    SessionBusy,
    InvalidLimits,
    ScanBudgetExhausted,
    Protocol,
    Page(PageValidationError),
}

impl fmt::Display for RedisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Connect => "Redis connection failed",
            Self::Command => "Redis command failed",
            Self::ServerCancelled => "Redis server confirmed client unblocking",
            Self::SessionBusy => "Redis session already owns a blocking operation",
            Self::InvalidLimits => "Redis stream limits are invalid",
            Self::ScanBudgetExhausted => "Redis scan round budget was exhausted",
            Self::Protocol => "Redis returned an unsupported response",
            Self::Page(_) => "Redis result page failed validation",
        })
    }
}

impl Error for RedisError {}

pub struct RedisSession {
    connection: MultiplexedConnection,
    control: MultiplexedConnection,
    client_id: u64,
    blocking: Arc<RedisBlockingState>,
}

#[derive(Default)]
struct RedisBlockingState {
    active: AtomicBool,
    server_confirmed: AtomicBool,
}

impl RedisSession {
    pub async fn connect(config: &RedisConnectConfig) -> Result<Self, RedisError> {
        let addr = match config.tls {
            RedisTlsMode::Disable => {
                ConnectionAddr::Tcp(config.host.as_str().to_owned(), config.port)
            }
            RedisTlsMode::Require => ConnectionAddr::TcpTls {
                host: config.host.as_str().to_owned(),
                port: config.port,
                insecure: false,
                tls_params: None,
            },
        };
        let protocol = match config.protocol {
            RedisProtocol::Resp2 => ProtocolVersion::RESP2,
            RedisProtocol::Resp3 => ProtocolVersion::RESP3,
        };
        let redis = RedisConnectionInfo::default()
            .set_db(config.database)
            .set_protocol(protocol)
            .set_lib_name("tablerock", env!("CARGO_PKG_VERSION"));
        let info = addr
            .into_connection_info()
            .map_err(|_| RedisError::Connect)?
            .set_redis_settings(redis);
        let client = Client::open(info).map_err(|_| RedisError::Connect)?;
        let mut connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| RedisError::Connect)?;
        let client_id: u64 = redis::cmd("CLIENT")
            .arg("ID")
            .query_async(&mut connection)
            .await
            .map_err(|_| RedisError::Connect)?;
        let control = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| RedisError::Connect)?;
        Ok(Self {
            connection,
            control,
            client_id,
            blocking: Arc::new(RedisBlockingState::default()),
        })
    }

    #[must_use]
    pub const fn client_id(&self) -> u64 {
        self.client_id
    }

    pub fn blocking_pop(
        &self,
        key: BoundedBytes,
        limits: PageLimits,
        max_cell_bytes: u64,
    ) -> Result<RedisBlockingPopStream, RedisError> {
        if key.is_empty()
            || limits.max_rows() == 0
            || limits.max_columns() < 2
            || limits.max_arena_bytes() == 0
            || max_cell_bytes == 0
        {
            return Err(RedisError::InvalidLimits);
        }
        if self.blocking.active.swap(true, Ordering::AcqRel) {
            return Err(RedisError::SessionBusy);
        }
        self.blocking
            .server_confirmed
            .store(false, Ordering::Release);
        let mut connection = self.connection.clone();
        let command_key = key.clone();
        let (result_tx, result_rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let result = redis::cmd("BLPOP")
                .arg(command_key.as_slice())
                .arg(0)
                .query_async(&mut connection)
                .await
                .map_err(|_| RedisError::Command);
            let _ = result_tx.send(result);
        });
        Ok(RedisBlockingPopStream {
            result: result_rx,
            task,
            limits,
            max_cell_bytes,
            complete: false,
            blocking: Arc::clone(&self.blocking),
        })
    }

    pub async fn dispatch_cancel(&self) -> Result<bool, RedisError> {
        if !self.blocking.active.load(Ordering::Acquire) {
            return Ok(false);
        }
        let mut control = self.control.clone();
        let unblocked: u64 = redis::cmd("CLIENT")
            .arg("UNBLOCK")
            .arg(self.client_id)
            .arg("ERROR")
            .query_async(&mut control)
            .await
            .map_err(|_| RedisError::Command)?;
        if unblocked == 1 {
            self.blocking
                .server_confirmed
                .store(true, Ordering::Release);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn scan_keys(
        &self,
        limits: PageLimits,
        max_cell_bytes: u64,
        scan_count: u32,
        max_scan_rounds: u32,
    ) -> Result<RedisKeyStream, RedisError> {
        if limits.max_rows() == 0
            || limits.max_columns() == 0
            || limits.max_arena_bytes() == 0
            || max_cell_bytes == 0
            || scan_count == 0
            || max_scan_rounds == 0
        {
            return Err(RedisError::InvalidLimits);
        }
        Ok(RedisKeyStream {
            connection: self.connection.clone(),
            cursor: 0,
            pending: VecDeque::new(),
            started: false,
            complete: false,
            limits,
            max_cell_bytes,
            scan_count,
            remaining_rounds: max_scan_rounds,
        })
    }

    pub async fn negotiated_protocol(&self) -> Result<RedisProtocol, RedisError> {
        let mut connection = self.connection.clone();
        let info: redis::Value = redis::cmd("CLIENT")
            .arg("INFO")
            .query_async(&mut connection)
            .await
            .map_err(|_| RedisError::Command)?;
        let info = match &info {
            redis::Value::BulkString(value) => value.as_slice(),
            redis::Value::SimpleString(value)
            | redis::Value::VerbatimString { text: value, .. } => value.as_bytes(),
            _ => return Err(RedisError::Protocol),
        };
        if info
            .split(|byte| *byte == b' ' || *byte == b'\n' || *byte == b'\r')
            .any(|field| field == b"resp=2")
        {
            Ok(RedisProtocol::Resp2)
        } else if info
            .split(|byte| *byte == b' ' || *byte == b'\n' || *byte == b'\r')
            .any(|field| field == b"resp=3")
        {
            Ok(RedisProtocol::Resp3)
        } else {
            Err(RedisError::Protocol)
        }
    }

    pub async fn read_binary(
        &self,
        key: &BoundedBytes,
        max_bytes: u64,
    ) -> Result<Option<OwnedValue>, RedisError> {
        if max_bytes == 0 {
            return Err(RedisError::InvalidLimits);
        }
        let mut connection = self.connection.clone();
        let value: Option<Vec<u8>> = redis::cmd("GET")
            .arg(key.as_slice())
            .query_async(&mut connection)
            .await
            .map_err(|_| RedisError::Command)?;
        value
            .map(|value| bounded_binary(&value, max_bytes))
            .transpose()
    }
}

type RedisBlockingResult = Result<(Vec<u8>, Vec<u8>), RedisError>;

pub struct RedisBlockingPopStream {
    result: tokio::sync::oneshot::Receiver<RedisBlockingResult>,
    task: tokio::task::JoinHandle<()>,
    limits: PageLimits,
    max_cell_bytes: u64,
    complete: bool,
    blocking: Arc<RedisBlockingState>,
}

impl RedisBlockingPopStream {
    pub async fn next_page(
        &mut self,
        identity: PageIdentity,
        start_row: u64,
    ) -> Result<Option<ResultPage>, RedisError> {
        if self.complete {
            return Ok(None);
        }
        let result = (&mut self.result).await.map_err(|_| RedisError::Command)?;
        self.complete = true;
        self.blocking.active.store(false, Ordering::Release);
        let (key, value) = match result {
            Ok(value) => value,
            Err(_) if self.blocking.server_confirmed.load(Ordering::Acquire) => {
                return Err(RedisError::ServerCancelled);
            }
            Err(_) => return Err(RedisError::Command),
        };
        let key = bounded_binary(&key, self.max_cell_bytes)?;
        let remaining = self
            .limits
            .max_arena_bytes()
            .saturating_sub(key.encoded_byte_len());
        let value = bounded_binary(&value, self.max_cell_bytes.min(remaining))?;
        let columns = ["key", "value"]
            .into_iter()
            .map(|name| {
                Ok(ColumnMetadata::new(
                    BoundedText::copy_from_str(name, ByteLimit::new(name.len() as u64))
                        .map_err(|_| RedisError::Protocol)?,
                    EngineType::new(
                        Engine::Redis,
                        BoundedText::copy_from_str("bulk-string", ByteLimit::new(11))
                            .map_err(|_| RedisError::Protocol)?,
                    )
                    .map_err(|_| RedisError::Protocol)?,
                    false,
                ))
            })
            .collect::<Result<Vec<_>, RedisError>>()?;
        ResultPage::from_row_major(
            identity,
            start_row,
            RowTotal::Known(1),
            PageFacts::new(PageDelivery::Final, PageWarnings::none()),
            columns,
            vec![key, value],
            self.limits,
        )
        .map(Some)
        .map_err(RedisError::Page)
    }
}

impl Drop for RedisBlockingPopStream {
    fn drop(&mut self) {
        self.task.abort();
        self.blocking.active.store(false, Ordering::Release);
    }
}

pub struct RedisKeyStream {
    connection: MultiplexedConnection,
    cursor: u64,
    pending: VecDeque<Vec<u8>>,
    started: bool,
    complete: bool,
    limits: PageLimits,
    max_cell_bytes: u64,
    scan_count: u32,
    remaining_rounds: u32,
}

impl RedisKeyStream {
    pub async fn next_page(
        &mut self,
        identity: PageIdentity,
        start_row: u64,
    ) -> Result<Option<ResultPage>, RedisError> {
        if self.complete {
            return Ok(None);
        }
        let mut values = Vec::new();
        let mut arena_remaining = self.limits.max_arena_bytes();
        while values.len() < self.limits.max_rows() as usize {
            if let Some(key) = self.pending.pop_front() {
                let limit = self.max_cell_bytes.min(arena_remaining);
                let value = bounded_binary(&key, limit)?;
                arena_remaining = arena_remaining.saturating_sub(value.encoded_byte_len());
                values.push(value);
                continue;
            }
            if self.started && self.cursor == 0 {
                self.complete = true;
                break;
            }
            if self.remaining_rounds == 0 {
                if values.is_empty() {
                    return Err(RedisError::ScanBudgetExhausted);
                }
                break;
            }
            let (cursor, keys): (u64, Vec<Vec<u8>>) = redis::cmd("SCAN")
                .arg(self.cursor)
                .arg("COUNT")
                .arg(self.scan_count)
                .query_async(&mut self.connection)
                .await
                .map_err(|_| RedisError::Command)?;
            self.started = true;
            self.cursor = cursor;
            self.remaining_rounds -= 1;
            self.pending.extend(keys);
        }
        if self.started && self.cursor == 0 && self.pending.is_empty() {
            self.complete = true;
        }
        let final_page = self.complete;
        let delivery = if final_page {
            PageDelivery::Final
        } else {
            PageDelivery::Partial
        };
        let mut warnings = PageWarnings::none();
        if !final_page
            && (values.len() == self.limits.max_rows() as usize || !self.pending.is_empty())
        {
            warnings = warnings.with(PageWarning::RowLimitReached);
        }
        if values.iter().any(OwnedValue::is_truncated) {
            warnings = warnings.with(PageWarning::ByteLimitReached);
        }
        let columns = vec![ColumnMetadata::new(
            BoundedText::copy_from_str("key", ByteLimit::new(3))
                .map_err(|_| RedisError::Protocol)?,
            EngineType::new(
                Engine::Redis,
                BoundedText::copy_from_str("bulk-string", ByteLimit::new(11))
                    .map_err(|_| RedisError::Protocol)?,
            )
            .map_err(|_| RedisError::Protocol)?,
            false,
        )];
        ResultPage::from_row_major(
            identity,
            start_row,
            RowTotal::Unknown,
            PageFacts::new(delivery, warnings),
            columns,
            values,
            self.limits,
        )
        .map(Some)
        .map_err(RedisError::Page)
    }
}

fn bounded_binary(value: &[u8], limit: u64) -> Result<OwnedValue, RedisError> {
    let stored_len = usize::try_from(limit)
        .unwrap_or(usize::MAX)
        .min(value.len());
    let bytes = BoundedBytes::copy_from_slice(&value[..stored_len], ByteLimit::new(limit))
        .map_err(|_| RedisError::Protocol)?;
    let truncation = if stored_len == value.len() {
        Truncation::Complete
    } else {
        Truncation::Truncated {
            original_byte_len: Some(value.len() as u64),
        }
    };
    OwnedValue::binary(bytes, truncation).map_err(|_| RedisError::Protocol)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_debug_redacts_host_text() {
        let config = RedisConnectConfig::new(
            BoundedText::copy_from_str("SECRET_HOST", ByteLimit::new(64)).unwrap(),
            6379,
            0,
            RedisProtocol::Resp3,
            RedisTlsMode::Require,
        );
        let debug = format!("{config:?}");
        assert!(!debug.contains("SECRET_HOST"));
        assert!(debug.contains("Resp3"));
        assert!(debug.contains("Require"));
    }
}
