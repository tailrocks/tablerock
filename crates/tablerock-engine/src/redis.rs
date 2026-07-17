use std::{
    collections::VecDeque,
    error::Error,
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

use redis::{
    AsyncConnectionConfig, Client, ConnectionAddr, IntoConnectionInfo, ProtocolVersion,
    RedisConnectionInfo,
    aio::{ConnectionManager, ConnectionManagerConfig},
};
use tablerock_core::{
    BoundedBytes, BoundedText, ByteLimit, ColumnMetadata, Engine, EngineType, OwnedValue,
    PageDelivery, PageFacts, PageIdentity, PageLimits, PageValidationError, PageWarning,
    PageWarnings, RedisTimeToLive, ResultPage, RowTotal, Truncation,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RedisCollectionScanKind {
    Hash,
    Set,
    SortedSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RedisCollectionScanOptions {
    limits: PageLimits,
    max_cell_bytes: u64,
    scan_count: u32,
    max_batch_entries: u32,
    max_batch_bytes: u64,
    max_scan_rounds: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RedisRuntimePolicy {
    connection_timeout: Duration,
    response_timeout: Duration,
    reconnect_attempts: usize,
    reconnect_min_delay: Duration,
    reconnect_max_delay: Duration,
}

impl RedisRuntimePolicy {
    pub const MAX_DURATION: Duration = Duration::from_secs(300);

    pub fn new(
        connection_timeout: Duration,
        response_timeout: Duration,
        reconnect_attempts: usize,
        reconnect_min_delay: Duration,
        reconnect_max_delay: Duration,
    ) -> Result<Self, RedisError> {
        if connection_timeout.is_zero()
            || connection_timeout > Self::MAX_DURATION
            || response_timeout.is_zero()
            || response_timeout > Self::MAX_DURATION
            || reconnect_attempts == 0
            || reconnect_attempts > 32
            || reconnect_min_delay.is_zero()
            || reconnect_min_delay > Self::MAX_DURATION
            || reconnect_max_delay < reconnect_min_delay
            || reconnect_max_delay > Self::MAX_DURATION
        {
            return Err(RedisError::InvalidLimits);
        }
        Ok(Self {
            connection_timeout,
            response_timeout,
            reconnect_attempts,
            reconnect_min_delay,
            reconnect_max_delay,
        })
    }

    #[must_use]
    pub const fn connection_timeout(self) -> Duration {
        self.connection_timeout
    }

    #[must_use]
    pub const fn response_timeout(self) -> Duration {
        self.response_timeout
    }

    #[must_use]
    pub const fn reconnect_attempts(self) -> usize {
        self.reconnect_attempts
    }

    #[must_use]
    pub const fn reconnect_min_delay(self) -> Duration {
        self.reconnect_min_delay
    }

    #[must_use]
    pub const fn reconnect_max_delay(self) -> Duration {
        self.reconnect_max_delay
    }

    fn manager_config(self) -> ConnectionManagerConfig {
        ConnectionManagerConfig::new()
            .set_connection_timeout(Some(self.connection_timeout))
            .set_response_timeout(Some(self.response_timeout))
            .set_number_of_retries(self.reconnect_attempts)
            .set_min_delay(self.reconnect_min_delay)
            .set_max_delay(self.reconnect_max_delay)
            .set_pipeline_buffer_size(50)
            .set_concurrency_limit(64)
    }

    fn blocking_config(self) -> AsyncConnectionConfig {
        AsyncConnectionConfig::new()
            .set_connection_timeout(Some(self.connection_timeout))
            .set_response_timeout(None)
            .set_pipeline_buffer_size(1)
            .set_concurrency_limit(1)
    }
}

impl Default for RedisRuntimePolicy {
    fn default() -> Self {
        Self {
            connection_timeout: Duration::from_secs(5),
            response_timeout: Duration::from_secs(30),
            reconnect_attempts: 8,
            reconnect_min_delay: Duration::from_millis(100),
            reconnect_max_delay: Duration::from_secs(2),
        }
    }
}

impl RedisCollectionScanOptions {
    #[must_use]
    pub const fn new(
        limits: PageLimits,
        max_cell_bytes: u64,
        scan_count: u32,
        max_batch_entries: u32,
        max_batch_bytes: u64,
        max_scan_rounds: u32,
    ) -> Self {
        Self {
            limits,
            max_cell_bytes,
            scan_count,
            max_batch_entries,
            max_batch_bytes,
            max_scan_rounds,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RedisConnectConfig {
    host: BoundedText,
    port: u16,
    database: i64,
    protocol: RedisProtocol,
    tls: RedisTlsMode,
    runtime_policy: RedisRuntimePolicy,
}

impl RedisConnectConfig {
    #[must_use]
    pub fn new(
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
            runtime_policy: RedisRuntimePolicy::default(),
        }
    }

    #[must_use]
    pub const fn with_runtime_policy(mut self, runtime_policy: RedisRuntimePolicy) -> Self {
        self.runtime_policy = runtime_policy;
        self
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
            .field("runtime_policy", &self.runtime_policy)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedisError {
    Connect,
    Connection,
    Timeout,
    Command,
    ClientCancelled,
    ServerCancelled,
    SessionBusy,
    InvalidLimits,
    ScanBudgetExhausted,
    ScanResponseLimitExceeded,
    Protocol,
    Page(PageValidationError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedisCancelDispatch {
    PreventedBeforeDispatch,
    RequestSent,
    ServerRejected,
}

impl fmt::Display for RedisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Connect => "Redis connection failed",
            Self::Connection => "Redis connection was lost",
            Self::Timeout => "Redis operation timed out",
            Self::Command => "Redis command failed",
            Self::ClientCancelled => "Redis operation was cancelled before dispatch",
            Self::ServerCancelled => "Redis server confirmed client unblocking",
            Self::SessionBusy => "Redis session already owns a blocking operation",
            Self::InvalidLimits => "Redis stream limits are invalid",
            Self::ScanBudgetExhausted => "Redis scan round budget was exhausted",
            Self::ScanResponseLimitExceeded => "Redis scan response exceeded its safety bound",
            Self::Protocol => "Redis returned an unsupported response",
            Self::Page(_) => "Redis result page failed validation",
        })
    }
}

impl Error for RedisError {}

pub struct RedisSession {
    client: Client,
    connection: ConnectionManager,
    control: ConnectionManager,
    runtime_policy: RedisRuntimePolicy,
    blocking: Arc<RedisBlockingRegistry>,
}

#[derive(Default)]
struct RedisBlockingRegistry {
    active: Mutex<Option<Arc<RedisBlockingOperation>>>,
}

#[derive(Default)]
struct RedisBlockingOperation {
    client_id: AtomicU64,
    server_confirmed: AtomicBool,
    cancel_requested: AtomicBool,
}

impl RedisBlockingRegistry {
    fn claim(&self) -> Result<Arc<RedisBlockingOperation>, RedisError> {
        let mut active = self
            .active
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if active.is_some() {
            return Err(RedisError::SessionBusy);
        }
        let operation = Arc::new(RedisBlockingOperation::default());
        *active = Some(Arc::clone(&operation));
        Ok(operation)
    }

    fn active(&self) -> Option<Arc<RedisBlockingOperation>> {
        self.active
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    fn is_active(&self, operation: &Arc<RedisBlockingOperation>) -> bool {
        self.active
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, operation))
    }

    fn release(&self, operation: &Arc<RedisBlockingOperation>) {
        let mut active = self
            .active
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if active
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, operation))
        {
            *active = None;
        }
    }
}

struct RedisBlockingClaim {
    registry: Arc<RedisBlockingRegistry>,
    operation: Arc<RedisBlockingOperation>,
    armed: bool,
}

impl RedisBlockingClaim {
    fn new(registry: Arc<RedisBlockingRegistry>) -> Result<Self, RedisError> {
        let operation = registry.claim()?;
        Ok(Self {
            registry,
            operation,
            armed: true,
        })
    }

    fn commit(mut self) -> Arc<RedisBlockingOperation> {
        self.armed = false;
        Arc::clone(&self.operation)
    }
}

impl Drop for RedisBlockingClaim {
    fn drop(&mut self) {
        if self.armed {
            self.registry.release(&self.operation);
        }
    }
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
        let connection = ConnectionManager::new_with_config(
            client.clone(),
            config.runtime_policy.manager_config(),
        )
        .await
        .map_err(map_connect_error)?;
        let control = ConnectionManager::new_with_config(
            client.clone(),
            config.runtime_policy.manager_config(),
        )
        .await
        .map_err(map_connect_error)?;
        Ok(Self {
            client,
            connection,
            control,
            runtime_policy: config.runtime_policy,
            blocking: Arc::new(RedisBlockingRegistry::default()),
        })
    }

    #[must_use]
    pub fn active_blocking_client_id(&self) -> Option<u64> {
        let client_id = self.blocking.active()?.client_id.load(Ordering::Acquire);
        (client_id != 0).then_some(client_id)
    }

    pub async fn observed_client_id(&self) -> Result<u64, RedisError> {
        let mut connection = self.connection.clone();
        redis::cmd("CLIENT")
            .arg("ID")
            .query_async(&mut connection)
            .await
            .map_err(map_command_error)
    }

    pub async fn blocking_pop(
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
        let claim = RedisBlockingClaim::new(Arc::clone(&self.blocking))?;
        let mut connection = match self
            .client
            .get_multiplexed_async_connection_with_config(&self.runtime_policy.blocking_config())
            .await
        {
            Ok(connection) => connection,
            Err(error) => return Err(map_connect_error(error)),
        };
        let mut identity_command = redis::cmd("CLIENT");
        identity_command.arg("ID");
        let identity = identity_command.query_async(&mut connection);
        let client_id: u64 =
            match tokio::time::timeout(self.runtime_policy.response_timeout(), identity).await {
                Ok(Ok(client_id)) => client_id,
                Ok(Err(error)) => return Err(map_command_error(error)),
                Err(_) => return Err(RedisError::Timeout),
            };
        claim
            .operation
            .client_id
            .store(client_id, Ordering::Release);
        if claim.operation.cancel_requested.load(Ordering::Acquire) {
            return Err(RedisError::ClientCancelled);
        }
        let command_key = key.clone();
        let (result_tx, result_rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let result = redis::cmd("BLPOP")
                .arg(command_key.as_slice())
                .arg(0)
                .query_async(&mut connection)
                .await
                .map_err(map_command_error);
            let _ = result_tx.send(result);
        });
        let operation = claim.commit();
        Ok(RedisBlockingPopStream {
            result: result_rx,
            task,
            limits,
            max_cell_bytes,
            complete: false,
            blocking: Arc::clone(&self.blocking),
            operation,
        })
    }

    pub async fn dispatch_cancel(&self) -> Result<RedisCancelDispatch, RedisError> {
        let Some(operation) = self.blocking.active() else {
            return Ok(RedisCancelDispatch::ServerRejected);
        };
        operation.cancel_requested.store(true, Ordering::Release);
        let client_id = operation.client_id.load(Ordering::Acquire);
        if client_id == 0 {
            return Ok(RedisCancelDispatch::PreventedBeforeDispatch);
        }
        let deadline = tokio::time::Instant::now() + self.runtime_policy.response_timeout();
        loop {
            if !self.blocking.is_active(&operation) {
                return Ok(RedisCancelDispatch::ServerRejected);
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(RedisError::Timeout);
            }
            let mut control = self.control.clone();
            let mut unblock_command = redis::cmd("CLIENT");
            unblock_command.arg("UNBLOCK").arg(client_id).arg("ERROR");
            let request = unblock_command.query_async(&mut control);
            let unblocked: u64 = tokio::time::timeout(remaining, request)
                .await
                .map_err(|_| RedisError::Timeout)?
                .map_err(map_command_error)?;
            if unblocked == 1 {
                operation.server_confirmed.store(true, Ordering::Release);
                return Ok(RedisCancelDispatch::RequestSent);
            }
            if !self.blocking.is_active(&operation) {
                return Ok(RedisCancelDispatch::ServerRejected);
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(RedisError::Timeout);
            }
            tokio::time::sleep(Duration::from_millis(5).min(remaining)).await;
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
            emitted_page: false,
            limits,
            max_cell_bytes,
            scan_count,
            remaining_rounds: max_scan_rounds,
        })
    }

    pub fn scan_collection(
        &self,
        key: BoundedBytes,
        kind: RedisCollectionScanKind,
        options: RedisCollectionScanOptions,
    ) -> Result<RedisCollectionStream, RedisError> {
        validate_collection_limits(kind, options)?;
        Ok(RedisCollectionStream {
            connection: self.connection.clone(),
            key,
            kind,
            cursor: 0,
            pending: VecDeque::new(),
            started: false,
            complete: false,
            emitted_page: false,
            limits: options.limits,
            max_cell_bytes: options.max_cell_bytes,
            scan_count: options.scan_count,
            max_batch_entries: options.max_batch_entries,
            max_batch_bytes: options.max_batch_bytes,
            remaining_rounds: options.max_scan_rounds,
        })
    }

    pub async fn negotiated_protocol(&self) -> Result<RedisProtocol, RedisError> {
        let mut connection = self.connection.clone();
        let info: redis::Value = redis::cmd("CLIENT")
            .arg("INFO")
            .query_async(&mut connection)
            .await
            .map_err(map_command_error)?;
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
            .map_err(map_command_error)?;
        value
            .map(|value| bounded_binary(&value, max_bytes))
            .transpose()
    }

    pub async fn read_time_to_live(
        &self,
        key: &BoundedBytes,
    ) -> Result<RedisTimeToLive, RedisError> {
        let mut connection = self.connection.clone();
        let remaining: i64 = redis::cmd("PTTL")
            .arg(key.as_slice())
            .query_async(&mut connection)
            .await
            .map_err(map_command_error)?;
        decode_time_to_live(remaining)
    }
}

fn map_connect_error(error: redis::RedisError) -> RedisError {
    if error.is_timeout() {
        RedisError::Timeout
    } else {
        RedisError::Connect
    }
}

fn map_command_error(error: redis::RedisError) -> RedisError {
    if error.is_timeout() {
        RedisError::Timeout
    } else if error.is_connection_dropped() || error.is_io_error() {
        RedisError::Connection
    } else {
        RedisError::Command
    }
}

fn validate_collection_limits(
    kind: RedisCollectionScanKind,
    options: RedisCollectionScanOptions,
) -> Result<(), RedisError> {
    let RedisCollectionScanOptions {
        limits,
        max_cell_bytes,
        scan_count,
        max_batch_entries,
        max_batch_bytes,
        max_scan_rounds,
    } = options;
    let required_columns = match kind {
        RedisCollectionScanKind::Set => 1,
        RedisCollectionScanKind::Hash | RedisCollectionScanKind::SortedSet => 2,
    };
    let required_column_text_bytes = match kind {
        RedisCollectionScanKind::Set => 17,
        RedisCollectionScanKind::Hash => 32,
        RedisCollectionScanKind::SortedSet => 28,
    };
    let score_arena_bytes = u64::from(limits.max_rows()).checked_mul(8);
    if limits.max_rows() == 0
        || limits.max_columns() < required_columns
        || limits.max_arena_bytes() == 0
        || limits.max_column_text_bytes() < required_column_text_bytes
        || (matches!(kind, RedisCollectionScanKind::SortedSet)
            && score_arena_bytes.is_none_or(|required| limits.max_arena_bytes() < required))
        || max_cell_bytes == 0
        || scan_count == 0
        || max_batch_entries == 0
        || max_batch_bytes == 0
        || max_scan_rounds == 0
    {
        return Err(RedisError::InvalidLimits);
    }
    Ok(())
}

enum RedisCollectionEntry {
    Binary(Vec<u8>),
    Pair(Vec<u8>, Vec<u8>),
    Scored(Vec<u8>, f64),
}

type RedisPairScanReply = (u64, Vec<(Vec<u8>, Vec<u8>)>);

pub struct RedisCollectionStream {
    connection: ConnectionManager,
    key: BoundedBytes,
    kind: RedisCollectionScanKind,
    cursor: u64,
    pending: VecDeque<RedisCollectionEntry>,
    started: bool,
    complete: bool,
    emitted_page: bool,
    limits: PageLimits,
    max_cell_bytes: u64,
    scan_count: u32,
    max_batch_entries: u32,
    max_batch_bytes: u64,
    remaining_rounds: u32,
}

impl RedisCollectionStream {
    pub async fn next_page(
        &mut self,
        identity: PageIdentity,
        start_row: u64,
    ) -> Result<Option<ResultPage>, RedisError> {
        if self.complete {
            return Ok(None);
        }
        let mut values = Vec::new();
        let mut rows = 0_u32;
        let mut arena_remaining = self.limits.max_arena_bytes();
        while rows < self.limits.max_rows() {
            if let Some(entry) = self.pending.pop_front() {
                append_collection_entry(
                    entry,
                    &mut values,
                    self.max_cell_bytes,
                    &mut arena_remaining,
                )?;
                rows += 1;
                continue;
            }
            if self.started && self.cursor == 0 {
                self.complete = true;
                break;
            }
            if self.remaining_rounds == 0 {
                if rows == 0 {
                    return Err(RedisError::ScanBudgetExhausted);
                }
                break;
            }
            let (cursor, entries) = match self.kind {
                RedisCollectionScanKind::Hash => {
                    let (cursor, entries): RedisPairScanReply = redis::cmd("HSCAN")
                        .arg(self.key.as_slice())
                        .arg(self.cursor)
                        .arg("COUNT")
                        .arg(self.scan_count)
                        .query_async(&mut self.connection)
                        .await
                        .map_err(map_command_error)?;
                    validate_scan_batch(
                        entries.len(),
                        entries.iter().try_fold(0_u64, |total, (field, value)| {
                            total
                                .checked_add(field.len() as u64)?
                                .checked_add(value.len() as u64)
                        }),
                        self.max_batch_entries,
                        self.max_batch_bytes,
                    )?;
                    (
                        cursor,
                        entries
                            .into_iter()
                            .map(|(field, value)| RedisCollectionEntry::Pair(field, value))
                            .collect::<Vec<_>>(),
                    )
                }
                RedisCollectionScanKind::Set => {
                    let (cursor, entries): (u64, Vec<Vec<u8>>) = redis::cmd("SSCAN")
                        .arg(self.key.as_slice())
                        .arg(self.cursor)
                        .arg("COUNT")
                        .arg(self.scan_count)
                        .query_async(&mut self.connection)
                        .await
                        .map_err(map_command_error)?;
                    validate_scan_batch(
                        entries.len(),
                        entries.iter().try_fold(0_u64, |total, member| {
                            total.checked_add(member.len() as u64)
                        }),
                        self.max_batch_entries,
                        self.max_batch_bytes,
                    )?;
                    (
                        cursor,
                        entries
                            .into_iter()
                            .map(RedisCollectionEntry::Binary)
                            .collect::<Vec<_>>(),
                    )
                }
                RedisCollectionScanKind::SortedSet => {
                    let (cursor, entries): RedisPairScanReply = redis::cmd("ZSCAN")
                        .arg(self.key.as_slice())
                        .arg(self.cursor)
                        .arg("COUNT")
                        .arg(self.scan_count)
                        .query_async(&mut self.connection)
                        .await
                        .map_err(map_command_error)?;
                    validate_scan_batch(
                        entries.len(),
                        entries.iter().try_fold(0_u64, |total, (member, score)| {
                            total
                                .checked_add(member.len() as u64)?
                                .checked_add(score.len() as u64)
                        }),
                        self.max_batch_entries,
                        self.max_batch_bytes,
                    )?;
                    let entries = entries
                        .into_iter()
                        .map(|(member, score)| {
                            let score = std::str::from_utf8(&score)
                                .map_err(|_| RedisError::Protocol)?
                                .parse::<f64>()
                                .map_err(|_| RedisError::Protocol)?;
                            Ok(RedisCollectionEntry::Scored(member, score))
                        })
                        .collect::<Result<Vec<_>, RedisError>>()?;
                    (cursor, entries)
                }
            };
            self.started = true;
            self.cursor = cursor;
            self.remaining_rounds -= 1;
            self.pending.extend(entries);
        }
        if self.started && self.cursor == 0 && self.pending.is_empty() {
            self.complete = true;
        }
        if rows == 0 && self.complete && self.emitted_page {
            return Ok(None);
        }
        let delivery = if self.complete {
            PageDelivery::Final
        } else {
            PageDelivery::Partial
        };
        let mut warnings = PageWarnings::none();
        if !self.complete && (rows == self.limits.max_rows() || !self.pending.is_empty()) {
            warnings = warnings.with(PageWarning::RowLimitReached);
        }
        if values.iter().any(OwnedValue::is_truncated) {
            warnings = warnings.with(PageWarning::ByteLimitReached);
        }
        let page = ResultPage::from_row_major(
            identity,
            start_row,
            RowTotal::Unknown,
            PageFacts::new(delivery, warnings),
            collection_columns(self.kind)?,
            values,
            self.limits,
        )
        .map_err(RedisError::Page)?;
        self.emitted_page = true;
        Ok(Some(page))
    }
}

fn validate_scan_batch(
    entry_count: usize,
    encoded_bytes: Option<u64>,
    max_entries: u32,
    max_bytes: u64,
) -> Result<(), RedisError> {
    if entry_count > max_entries as usize || encoded_bytes.is_none_or(|bytes| bytes > max_bytes) {
        return Err(RedisError::ScanResponseLimitExceeded);
    }
    Ok(())
}

fn append_collection_entry(
    entry: RedisCollectionEntry,
    values: &mut Vec<OwnedValue>,
    max_cell_bytes: u64,
    arena_remaining: &mut u64,
) -> Result<(), RedisError> {
    match entry {
        RedisCollectionEntry::Binary(value) => {
            let value = bounded_binary(&value, max_cell_bytes.min(*arena_remaining))?;
            *arena_remaining = arena_remaining.saturating_sub(value.encoded_byte_len());
            values.push(value);
        }
        RedisCollectionEntry::Pair(first, second) => {
            let first = bounded_binary(&first, max_cell_bytes.min(*arena_remaining))?;
            *arena_remaining = arena_remaining.saturating_sub(first.encoded_byte_len());
            let second = bounded_binary(&second, max_cell_bytes.min(*arena_remaining))?;
            *arena_remaining = arena_remaining.saturating_sub(second.encoded_byte_len());
            values.extend([first, second]);
        }
        RedisCollectionEntry::Scored(member, score) => {
            if *arena_remaining < 8 {
                return Err(RedisError::InvalidLimits);
            }
            let member_budget = arena_remaining.saturating_sub(8).min(max_cell_bytes);
            let member = bounded_binary(&member, member_budget)?;
            *arena_remaining = arena_remaining.saturating_sub(member.encoded_byte_len() + 8);
            values.extend([member, OwnedValue::float64_bits(score.to_bits())]);
        }
    }
    Ok(())
}

fn collection_columns(kind: RedisCollectionScanKind) -> Result<Vec<ColumnMetadata>, RedisError> {
    let columns = match kind {
        RedisCollectionScanKind::Hash => vec![("field", "bulk-string"), ("value", "bulk-string")],
        RedisCollectionScanKind::Set => vec![("member", "bulk-string")],
        RedisCollectionScanKind::SortedSet => {
            vec![("member", "bulk-string"), ("score", "double")]
        }
    };
    columns
        .into_iter()
        .map(|(name, data_type)| {
            Ok(ColumnMetadata::new(
                BoundedText::copy_from_str(name, ByteLimit::new(name.len() as u64))
                    .map_err(|_| RedisError::Protocol)?,
                EngineType::new(
                    Engine::Redis,
                    BoundedText::copy_from_str(data_type, ByteLimit::new(data_type.len() as u64))
                        .map_err(|_| RedisError::Protocol)?,
                )
                .map_err(|_| RedisError::Protocol)?,
                false,
            ))
        })
        .collect()
}

fn decode_time_to_live(remaining: i64) -> Result<RedisTimeToLive, RedisError> {
    match remaining {
        -2 => Ok(RedisTimeToLive::Missing),
        -1 => Ok(RedisTimeToLive::Persistent),
        0.. => Ok(RedisTimeToLive::Expiring {
            remaining_millis: remaining as u64,
        }),
        _ => Err(RedisError::Protocol),
    }
}

type RedisBlockingResult = Result<(Vec<u8>, Vec<u8>), RedisError>;

pub struct RedisBlockingPopStream {
    result: tokio::sync::oneshot::Receiver<RedisBlockingResult>,
    task: tokio::task::JoinHandle<()>,
    limits: PageLimits,
    max_cell_bytes: u64,
    complete: bool,
    blocking: Arc<RedisBlockingRegistry>,
    operation: Arc<RedisBlockingOperation>,
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
        self.blocking.release(&self.operation);
        let (key, value) = match result {
            Ok(value) => value,
            Err(_) if self.operation.server_confirmed.load(Ordering::Acquire) => {
                return Err(RedisError::ServerCancelled);
            }
            Err(error) => return Err(error),
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
        self.blocking.release(&self.operation);
    }
}

pub struct RedisKeyStream {
    connection: ConnectionManager,
    cursor: u64,
    pending: VecDeque<Vec<u8>>,
    started: bool,
    complete: bool,
    emitted_page: bool,
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
                .map_err(map_command_error)?;
            self.started = true;
            self.cursor = cursor;
            self.remaining_rounds -= 1;
            self.pending.extend(keys);
        }
        if self.started && self.cursor == 0 && self.pending.is_empty() {
            self.complete = true;
        }
        if values.is_empty() && self.complete && self.emitted_page {
            return Ok(None);
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
        let page = ResultPage::from_row_major(
            identity,
            start_row,
            RowTotal::Unknown,
            PageFacts::new(delivery, warnings),
            columns,
            values,
            self.limits,
        )
        .map_err(RedisError::Page)?;
        self.emitted_page = true;
        Ok(Some(page))
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
    fn blocking_registry_rejects_stale_release_and_confirmation() {
        let registry = RedisBlockingRegistry::default();
        let old = registry.claim().unwrap();
        old.client_id.store(1, Ordering::Release);
        registry.release(&old);

        let current = registry.claim().unwrap();
        current.client_id.store(2, Ordering::Release);
        registry.release(&old);
        old.server_confirmed.store(true, Ordering::Release);

        let active = registry.active().unwrap();
        assert!(Arc::ptr_eq(&active, &current));
        assert!(!registry.is_active(&old));
        assert!(registry.is_active(&current));
        assert_eq!(active.client_id.load(Ordering::Acquire), 2);
        assert!(!active.server_confirmed.load(Ordering::Acquire));
    }

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

    #[test]
    fn runtime_policy_rejects_unbounded_or_invalid_reconnect_settings() {
        let valid = RedisRuntimePolicy::new(
            RedisRuntimePolicy::MAX_DURATION,
            RedisRuntimePolicy::MAX_DURATION,
            32,
            RedisRuntimePolicy::MAX_DURATION,
            RedisRuntimePolicy::MAX_DURATION,
        )
        .unwrap();
        assert_eq!(valid.connection_timeout(), RedisRuntimePolicy::MAX_DURATION);
        assert_eq!(valid.response_timeout(), RedisRuntimePolicy::MAX_DURATION);
        assert_eq!(valid.reconnect_attempts(), 32);
        assert_eq!(
            valid.reconnect_min_delay(),
            RedisRuntimePolicy::MAX_DURATION
        );
        assert_eq!(
            valid.reconnect_max_delay(),
            RedisRuntimePolicy::MAX_DURATION
        );
        let manager = valid.manager_config();
        assert_eq!(
            manager.connection_timeout(),
            Some(RedisRuntimePolicy::MAX_DURATION)
        );
        assert_eq!(
            manager.response_timeout(),
            Some(RedisRuntimePolicy::MAX_DURATION)
        );
        assert_eq!(manager.number_of_retries(), 32);
        assert_eq!(manager.min_delay(), RedisRuntimePolicy::MAX_DURATION);
        assert_eq!(manager.max_delay(), Some(RedisRuntimePolicy::MAX_DURATION));

        for invalid in [
            RedisRuntimePolicy::new(
                Duration::ZERO,
                Duration::from_millis(1),
                1,
                Duration::from_millis(1),
                Duration::from_millis(1),
            ),
            RedisRuntimePolicy::new(
                Duration::from_millis(1),
                Duration::ZERO,
                1,
                Duration::from_millis(1),
                Duration::from_millis(1),
            ),
            RedisRuntimePolicy::new(
                Duration::from_millis(1),
                Duration::from_millis(1),
                0,
                Duration::from_millis(1),
                Duration::from_millis(1),
            ),
            RedisRuntimePolicy::new(
                Duration::from_millis(1),
                Duration::from_millis(1),
                33,
                Duration::from_millis(1),
                Duration::from_millis(1),
            ),
            RedisRuntimePolicy::new(
                Duration::from_millis(1),
                Duration::from_millis(1),
                1,
                Duration::ZERO,
                Duration::from_millis(1),
            ),
            RedisRuntimePolicy::new(
                Duration::from_millis(1),
                Duration::from_millis(1),
                1,
                Duration::from_millis(2),
                Duration::from_millis(1),
            ),
            RedisRuntimePolicy::new(
                RedisRuntimePolicy::MAX_DURATION + Duration::from_nanos(1),
                Duration::from_millis(1),
                1,
                Duration::from_millis(1),
                Duration::from_millis(1),
            ),
            RedisRuntimePolicy::new(
                Duration::from_millis(1),
                RedisRuntimePolicy::MAX_DURATION + Duration::from_nanos(1),
                1,
                Duration::from_millis(1),
                Duration::from_millis(1),
            ),
            RedisRuntimePolicy::new(
                Duration::from_millis(1),
                Duration::from_millis(1),
                1,
                Duration::from_millis(1),
                RedisRuntimePolicy::MAX_DURATION + Duration::from_nanos(1),
            ),
        ] {
            assert_eq!(invalid, Err(RedisError::InvalidLimits));
        }
    }

    #[test]
    fn ttl_decoder_covers_every_sentinel_and_integer_boundary() {
        assert_eq!(decode_time_to_live(-2), Ok(RedisTimeToLive::Missing));
        assert_eq!(decode_time_to_live(-1), Ok(RedisTimeToLive::Persistent));
        assert_eq!(
            decode_time_to_live(0),
            Ok(RedisTimeToLive::Expiring {
                remaining_millis: 0
            })
        );
        assert_eq!(
            decode_time_to_live(i64::MAX),
            Ok(RedisTimeToLive::Expiring {
                remaining_millis: i64::MAX as u64
            })
        );
        for undocumented in [i64::MIN, -4, -3] {
            assert_eq!(decode_time_to_live(undocumented), Err(RedisError::Protocol));
        }
    }

    #[test]
    fn collection_scan_limits_match_each_result_shape() {
        for (kind, columns, arena_bytes, column_text_bytes) in [
            (RedisCollectionScanKind::Set, 1, 1, 17),
            (RedisCollectionScanKind::Hash, 2, 1, 32),
            (RedisCollectionScanKind::SortedSet, 2, 8, 28),
        ] {
            let options = |column_text_bytes| {
                RedisCollectionScanOptions::new(
                    PageLimits::new(1, columns, arena_bytes, column_text_bytes),
                    1,
                    1,
                    1,
                    1,
                    1,
                )
            };
            assert_eq!(
                validate_collection_limits(kind, options(column_text_bytes - 1)),
                Err(RedisError::InvalidLimits)
            );
            assert_eq!(
                validate_collection_limits(kind, options(column_text_bytes)),
                Ok(())
            );
        }
        assert_eq!(
            validate_collection_limits(
                RedisCollectionScanKind::Hash,
                RedisCollectionScanOptions::new(PageLimits::new(1, 1, 1, 32), 1, 1, 1, 1, 1),
            ),
            Err(RedisError::InvalidLimits)
        );
        assert_eq!(
            validate_collection_limits(
                RedisCollectionScanKind::SortedSet,
                RedisCollectionScanOptions::new(PageLimits::new(2, 2, 15, 28), 1, 1, 1, 1, 1),
            ),
            Err(RedisError::InvalidLimits)
        );
    }

    #[test]
    fn collection_scan_rejects_decoded_batches_above_either_bound() {
        assert_eq!(validate_scan_batch(2, Some(8), 2, 8), Ok(()));
        assert_eq!(
            validate_scan_batch(3, Some(8), 2, 8),
            Err(RedisError::ScanResponseLimitExceeded)
        );
        assert_eq!(
            validate_scan_batch(2, Some(9), 2, 8),
            Err(RedisError::ScanResponseLimitExceeded)
        );
        assert_eq!(
            validate_scan_batch(1, None, 2, u64::MAX),
            Err(RedisError::ScanResponseLimitExceeded)
        );
    }

    #[test]
    fn sorted_set_reserves_score_bytes_before_bounding_member() {
        let mut values = Vec::new();
        let mut arena_remaining = 10;
        append_collection_entry(
            RedisCollectionEntry::Scored(vec![1, 2, 3, 4], -1.25),
            &mut values,
            4,
            &mut arena_remaining,
        )
        .unwrap();
        assert_eq!(arena_remaining, 0);
        assert!(values[0].is_truncated());
        assert_eq!(values[0].encoded_byte_len(), 2);
        assert_eq!(values[1], OwnedValue::float64_bits((-1.25_f64).to_bits()));
    }
}
