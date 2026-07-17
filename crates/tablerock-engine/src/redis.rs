use std::future::Future;
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

use futures_util::StreamExt;
use redis::{
    AsyncConnectionConfig, Client, ClientTlsConfig, ConnectionAddr, ErrorKind, IntoConnectionInfo,
    ProtocolVersion, PushInfo, PushKind, RedisConnectionInfo, TlsCertificates,
    aio::{ConnectionManager, ConnectionManagerConfig},
};
use rustls::{
    RootCertStore,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
};

const MAX_REDIS_CREDENTIAL_BYTES: usize = 4_096;
const MAX_REDIS_TLS_MATERIAL_BYTES: usize = 65_536;
const MAX_REDIS_SUBSCRIPTION_MESSAGES: usize = 4_096;

#[derive(Clone, Copy)]
pub struct RedisCredentials<'a> {
    username: Option<&'a str>,
    password: &'a str,
}

impl<'a> RedisCredentials<'a> {
    #[must_use]
    pub const fn new(username: Option<&'a str>, password: &'a str) -> Self {
        Self { username, password }
    }
}

impl fmt::Debug for RedisCredentials<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RedisCredentials")
            .field("has_username", &self.username.is_some())
            .field("username_bytes", &self.username.map(str::len))
            .field("password_bytes", &self.password.len())
            .finish()
    }
}

#[derive(Clone, Copy)]
pub struct RedisClientIdentity<'a> {
    certificate_chain_pem: &'a [u8],
    private_key_pem: &'a [u8],
}

impl<'a> RedisClientIdentity<'a> {
    #[must_use]
    pub const fn new(certificate_chain_pem: &'a [u8], private_key_pem: &'a [u8]) -> Self {
        Self {
            certificate_chain_pem,
            private_key_pem,
        }
    }
}

impl fmt::Debug for RedisClientIdentity<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RedisClientIdentity")
            .field("certificate_chain_bytes", &self.certificate_chain_pem.len())
            .field("private_key_bytes", &self.private_key_pem.len())
            .finish()
    }
}

#[derive(Clone, Copy)]
pub struct RedisTlsMaterial<'a> {
    trust_roots: RedisTrustRoots<'a>,
    client_identity: Option<RedisClientIdentity<'a>>,
}

#[derive(Clone, Copy)]
enum RedisTrustRoots<'a> {
    Platform,
    Custom(&'a [u8]),
}

impl<'a> RedisTlsMaterial<'a> {
    #[must_use]
    pub const fn platform_roots() -> Self {
        Self {
            trust_roots: RedisTrustRoots::Platform,
            client_identity: None,
        }
    }

    #[must_use]
    pub const fn custom_roots(ca_certificates_pem: &'a [u8]) -> Self {
        Self {
            trust_roots: RedisTrustRoots::Custom(ca_certificates_pem),
            client_identity: None,
        }
    }

    #[must_use]
    pub const fn with_client_identity(mut self, identity: RedisClientIdentity<'a>) -> Self {
        self.client_identity = Some(identity);
        self
    }
}

impl fmt::Debug for RedisTlsMaterial<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RedisTlsMaterial")
            .field(
                "trust_roots",
                &match self.trust_roots {
                    RedisTrustRoots::Platform => "platform",
                    RedisTrustRoots::Custom(_) => "custom",
                },
            )
            .field("has_client_identity", &self.client_identity.is_some())
            .finish()
    }
}

#[derive(Clone, Copy)]
pub struct RedisConnectionSecurity<'a> {
    credentials: Option<RedisCredentials<'a>>,
    tls: Option<RedisTlsMaterial<'a>>,
}

impl<'a> RedisConnectionSecurity<'a> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            credentials: None,
            tls: None,
        }
    }

    #[must_use]
    pub const fn with_credentials(mut self, credentials: RedisCredentials<'a>) -> Self {
        self.credentials = Some(credentials);
        self
    }

    #[must_use]
    pub const fn with_tls(mut self, tls: RedisTlsMaterial<'a>) -> Self {
        self.tls = Some(tls);
        self
    }
}

impl Default for RedisConnectionSecurity<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for RedisConnectionSecurity<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RedisConnectionSecurity")
            .field("credentials", &self.credentials)
            .field("tls", &self.tls)
            .finish()
    }
}
use tablerock_core::{
    AuthorizedMutationPlan, BoundedBytes, BoundedText, ByteLimit, ColumnMetadata, Engine,
    EngineType, MutationChange, MutationId, MutationTarget, OwnedValue, PageDelivery, PageFacts,
    PageIdentity, PageLimits, PageValidationError, PageWarning, PageWarnings, RedisExpiration,
    RedisTimeToLive, ResultPage, ReviewTokenId, RowTotal, Truncation,
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

    fn handshake_config(self) -> AsyncConnectionConfig {
        AsyncConnectionConfig::new()
            .set_connection_timeout(Some(self.connection_timeout))
            .set_response_timeout(Some(self.response_timeout))
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
    database: u32,
    protocol: RedisProtocol,
    tls: RedisTlsMode,
    runtime_policy: RedisRuntimePolicy,
}

impl RedisConnectConfig {
    #[must_use]
    pub fn new(
        host: BoundedText,
        port: u16,
        database: u32,
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
    Authentication,
    TlsConfiguration,
    ClientCancelled,
    ServerCancelled,
    SessionBusy,
    InvalidLimits,
    ScanBudgetExhausted,
    ScanResponseLimitExceeded,
    SubscriptionOverflow,
    InvalidMutation,
    LogicalDatabaseMismatch,
    WriteOutcomeUnknown,
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
            Self::Authentication => "Redis authentication failed",
            Self::TlsConfiguration => "Redis TLS configuration is invalid",
            Self::ClientCancelled => "Redis operation was cancelled before dispatch",
            Self::ServerCancelled => "Redis server confirmed client unblocking",
            Self::SessionBusy => "Redis session already owns a long-lived operation",
            Self::InvalidLimits => "Redis stream limits are invalid",
            Self::ScanBudgetExhausted => "Redis scan round budget was exhausted",
            Self::ScanResponseLimitExceeded => "Redis scan response exceeded its safety bound",
            Self::SubscriptionOverflow => "Redis subscription buffer capacity was exceeded",
            Self::InvalidMutation => "Redis mutation plan is unsupported",
            Self::LogicalDatabaseMismatch => "Redis mutation targets another logical database",
            Self::WriteOutcomeUnknown => "Redis write outcome is unknown",
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
    subscription: Arc<RedisSubscriptionRegistry>,
    long_operation_active: Arc<AtomicBool>,
    protocol: RedisProtocol,
    logical_database: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedisTtlApplication {
    Applied,
    NotApplied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisTtlMutationOutcome {
    mutation_id: MutationId,
    review_token_id: ReviewTokenId,
    application: RedisTtlApplication,
}

impl RedisTtlMutationOutcome {
    #[must_use]
    pub const fn mutation_id(&self) -> MutationId {
        self.mutation_id
    }

    #[must_use]
    pub const fn review_token_id(&self) -> ReviewTokenId {
        self.review_token_id
    }

    #[must_use]
    pub const fn application(&self) -> RedisTtlApplication {
        self.application
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RedisSubscriptionOptions {
    limits: PageLimits,
    max_cell_bytes: u64,
    max_buffered_messages: usize,
}

impl RedisSubscriptionOptions {
    #[must_use]
    pub const fn new(
        limits: PageLimits,
        max_cell_bytes: u64,
        max_buffered_messages: usize,
    ) -> Self {
        Self {
            limits,
            max_cell_bytes,
            max_buffered_messages,
        }
    }
}

#[derive(Default)]
struct RedisSubscriptionRegistry {
    active: Mutex<Option<Arc<RedisSubscriptionOperation>>>,
}

#[derive(Default)]
struct RedisSubscriptionOperation {
    cancel_requested: AtomicBool,
    started: AtomicBool,
    wake: tokio::sync::Notify,
}

impl RedisSubscriptionRegistry {
    fn claim(&self) -> Result<Arc<RedisSubscriptionOperation>, RedisError> {
        let mut active = self
            .active
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if active.is_some() {
            return Err(RedisError::SessionBusy);
        }
        let operation = Arc::new(RedisSubscriptionOperation::default());
        *active = Some(Arc::clone(&operation));
        Ok(operation)
    }

    fn active(&self) -> Option<Arc<RedisSubscriptionOperation>> {
        self.active
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    fn release(&self, operation: &Arc<RedisSubscriptionOperation>) {
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

fn set_subscription_terminal(terminal: &Mutex<Option<RedisError>>, error: RedisError) -> bool {
    let mut current = terminal
        .lock()
        .unwrap_or_else(|failure| failure.into_inner());
    if current.is_some() {
        return false;
    }
    *current = Some(error);
    true
}

struct RedisSubscriptionClaim {
    registry: Arc<RedisSubscriptionRegistry>,
    operation: Arc<RedisSubscriptionOperation>,
    armed: bool,
    long_operation_active: Arc<AtomicBool>,
}

impl RedisSubscriptionClaim {
    fn new(
        registry: Arc<RedisSubscriptionRegistry>,
        long_operation_active: Arc<AtomicBool>,
    ) -> Result<Self, RedisError> {
        long_operation_active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| RedisError::SessionBusy)?;
        let operation = match registry.claim() {
            Ok(operation) => operation,
            Err(error) => {
                long_operation_active.store(false, Ordering::Release);
                return Err(error);
            }
        };
        Ok(Self {
            registry,
            operation,
            armed: true,
            long_operation_active,
        })
    }

    fn commit(mut self) {
        self.armed = false;
    }
}

impl Drop for RedisSubscriptionClaim {
    fn drop(&mut self) {
        if self.armed {
            self.registry.release(&self.operation);
            self.long_operation_active.store(false, Ordering::Release);
        }
    }
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
    long_operation_active: Arc<AtomicBool>,
}

impl RedisBlockingClaim {
    fn new(
        registry: Arc<RedisBlockingRegistry>,
        long_operation_active: Arc<AtomicBool>,
    ) -> Result<Self, RedisError> {
        long_operation_active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| RedisError::SessionBusy)?;
        let operation = match registry.claim() {
            Ok(operation) => operation,
            Err(error) => {
                long_operation_active.store(false, Ordering::Release);
                return Err(error);
            }
        };
        Ok(Self {
            registry,
            operation,
            armed: true,
            long_operation_active,
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
            self.long_operation_active.store(false, Ordering::Release);
        }
    }
}

async fn await_subscription_setup<T, F>(
    operation: &RedisSubscriptionOperation,
    future: F,
) -> Result<T, RedisError>
where
    F: Future<Output = Result<T, RedisError>>,
{
    if operation.cancel_requested.load(Ordering::Acquire) {
        return Err(RedisError::ClientCancelled);
    }
    tokio::pin!(future);
    tokio::select! {
        biased;
        () = operation.wake.notified() => Err(RedisError::ClientCancelled),
        result = &mut future => result,
    }
}

impl RedisSession {
    pub async fn connect(
        config: &RedisConnectConfig,
        security: RedisConnectionSecurity<'_>,
    ) -> Result<Self, RedisError> {
        validate_security(config.tls, security)?;
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
        let mut redis = RedisConnectionInfo::default()
            .set_db(i64::from(config.database))
            .set_protocol(protocol)
            .set_lib_name("tablerock", env!("CARGO_PKG_VERSION"));
        if let Some(credentials) = security.credentials {
            if let Some(username) = credentials.username {
                redis = redis.set_username(username);
            }
            redis = redis.set_password(credentials.password);
        }
        let info = addr
            .into_connection_info()
            .map_err(|_| RedisError::Connect)?
            .set_redis_settings(redis);
        let client = match security.tls {
            Some(tls) => Client::build_with_tls(
                info,
                TlsCertificates {
                    client_tls: tls.client_identity.map(|identity| ClientTlsConfig {
                        client_cert: identity.certificate_chain_pem.to_vec(),
                        client_key: identity.private_key_pem.to_vec(),
                    }),
                    root_cert: match tls.trust_roots {
                        RedisTrustRoots::Platform => None,
                        RedisTrustRoots::Custom(certificates) => Some(certificates.to_vec()),
                    },
                },
            )
            .map_err(map_client_build_error)?,
            None => Client::open(info).map_err(map_client_build_error)?,
        };
        client
            .get_multiplexed_async_connection_with_config(&config.runtime_policy.handshake_config())
            .await
            .map_err(map_connect_error)?;
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
            subscription: Arc::new(RedisSubscriptionRegistry::default()),
            long_operation_active: Arc::new(AtomicBool::new(false)),
            protocol: config.protocol,
            logical_database: config.database,
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
        let claim = RedisBlockingClaim::new(
            Arc::clone(&self.blocking),
            Arc::clone(&self.long_operation_active),
        )?;
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
            long_operation_active: Arc::clone(&self.long_operation_active),
        })
    }

    pub async fn dispatch_cancel(&self) -> Result<RedisCancelDispatch, RedisError> {
        if let Some(operation) = self.subscription.active() {
            operation.cancel_requested.store(true, Ordering::Release);
            operation.wake.notify_one();
            return Ok(if operation.started.load(Ordering::Acquire) {
                RedisCancelDispatch::RequestSent
            } else {
                RedisCancelDispatch::PreventedBeforeDispatch
            });
        }
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

    pub async fn subscribe(
        &self,
        channel: BoundedBytes,
        options: RedisSubscriptionOptions,
    ) -> Result<RedisSubscriptionStream, RedisError> {
        if channel.is_empty()
            || options.limits.max_rows() == 0
            || options.limits.max_columns() < 2
            || options.limits.max_arena_bytes() == 0
            || options.max_cell_bytes == 0
            || options.max_buffered_messages == 0
            || options.max_buffered_messages > MAX_REDIS_SUBSCRIPTION_MESSAGES
        {
            return Err(RedisError::InvalidLimits);
        }
        let claim = RedisSubscriptionClaim::new(
            Arc::clone(&self.subscription),
            Arc::clone(&self.long_operation_active),
        )?;
        let operation = Arc::clone(&claim.operation);
        let (messages_tx, messages_rx) = tokio::sync::mpsc::channel(options.max_buffered_messages);
        let terminal = Arc::new(Mutex::new(None));
        let terminal_wake = Arc::new(tokio::sync::Notify::new());
        let registry = Arc::clone(&self.subscription);
        let worker = match self.protocol {
            RedisProtocol::Resp2 => {
                let mut pubsub = await_subscription_setup(&operation, async {
                    self.client
                        .get_async_pubsub()
                        .await
                        .map_err(map_connect_error)
                })
                .await?;
                await_subscription_setup(&operation, async {
                    pubsub
                        .subscribe(channel.as_slice())
                        .await
                        .map_err(map_command_error)
                })
                .await?;
                operation.started.store(true, Ordering::Release);
                if operation.cancel_requested.load(Ordering::Acquire) {
                    operation.wake.notify_one();
                }
                let (mut sink, mut stream) = pubsub.split();
                let operation_worker = Arc::clone(&operation);
                let terminal_worker = Arc::clone(&terminal);
                let wake_worker = Arc::clone(&terminal_wake);
                let long_operation_worker = Arc::clone(&self.long_operation_active);
                let channel_worker = channel.clone();
                let teardown_timeout = self.runtime_policy.response_timeout();
                tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            () = operation_worker.wake.notified() => {
                                set_subscription_terminal(&terminal_worker, RedisError::ClientCancelled);
                                let _ = tokio::time::timeout(teardown_timeout, sink.unsubscribe(channel_worker.as_slice())).await;
                                break;
                            }
                            message = stream.next() => match message {
                                Some(message) => {
                                    let item = (message.get_channel::<Vec<u8>>(), message.get_payload::<Vec<u8>>());
                                    match item {
                                        (Ok(channel), Ok(payload)) => match messages_tx.try_send((channel, payload)) {
                                            Ok(()) => {}
                                            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                                                set_subscription_terminal(&terminal_worker, RedisError::SubscriptionOverflow);
                                                let _ = tokio::time::timeout(teardown_timeout, sink.unsubscribe(channel_worker.as_slice())).await;
                                                break;
                                            }
                                            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => break,
                                        },
                                        _ => {
                                            set_subscription_terminal(&terminal_worker, RedisError::Protocol);
                                            break;
                                        }
                                    }
                                }
                                None => {
                                    set_subscription_terminal(&terminal_worker, RedisError::Connection);
                                    break;
                                }
                            }
                        }
                    }
                    registry.release(&operation_worker);
                    long_operation_worker.store(false, Ordering::Release);
                    wake_worker.notify_one();
                })
            }
            RedisProtocol::Resp3 => {
                let disconnect = Arc::new(tokio::sync::Notify::new());
                let disconnect_sender = Arc::clone(&disconnect);
                let terminal_sender = Arc::clone(&terminal);
                let wake_sender = Arc::clone(&terminal_wake);
                let push_sender = move |push: PushInfo| -> Result<(), ()> {
                    match push.kind {
                        PushKind::Message if push.data.len() == 2 => {
                            let mut data = push.data.into_iter();
                            let decoded =
                                data.next().zip(data.next()).and_then(|(channel, payload)| {
                                    Some((
                                        redis::from_redis_value::<Vec<u8>>(channel).ok()?,
                                        redis::from_redis_value::<Vec<u8>>(payload).ok()?,
                                    ))
                                });
                            let Some((channel, payload)) = decoded else {
                                set_subscription_terminal(&terminal_sender, RedisError::Protocol);
                                disconnect_sender.notify_one();
                                wake_sender.notify_one();
                                return Err(());
                            };
                            messages_tx.try_send((channel, payload)).map_err(|error| {
                                let failure = match error {
                                    tokio::sync::mpsc::error::TrySendError::Full(_) => {
                                        RedisError::SubscriptionOverflow
                                    }
                                    tokio::sync::mpsc::error::TrySendError::Closed(_) => {
                                        RedisError::ClientCancelled
                                    }
                                };
                                set_subscription_terminal(&terminal_sender, failure);
                                disconnect_sender.notify_one();
                                wake_sender.notify_one();
                            })
                        }
                        PushKind::Disconnection => {
                            set_subscription_terminal(&terminal_sender, RedisError::Connection);
                            disconnect_sender.notify_one();
                            wake_sender.notify_one();
                            Ok(())
                        }
                        _ => Ok(()),
                    }
                };
                let config = self
                    .runtime_policy
                    .blocking_config()
                    .set_push_sender(push_sender);
                let mut connection = await_subscription_setup(&operation, async {
                    self.client
                        .get_multiplexed_async_connection_with_config(&config)
                        .await
                        .map_err(map_connect_error)
                })
                .await?;
                await_subscription_setup(&operation, async {
                    redis::cmd("SUBSCRIBE")
                        .arg(channel.as_slice())
                        .query_async::<()>(&mut connection)
                        .await
                        .map_err(map_command_error)
                })
                .await?;
                operation.started.store(true, Ordering::Release);
                if operation.cancel_requested.load(Ordering::Acquire) {
                    operation.wake.notify_one();
                }
                let operation_worker = Arc::clone(&operation);
                let terminal_worker = Arc::clone(&terminal);
                let wake_worker = Arc::clone(&terminal_wake);
                let long_operation_worker = Arc::clone(&self.long_operation_active);
                let channel_worker = channel.clone();
                let teardown_timeout = self.runtime_policy.response_timeout();
                tokio::spawn(async move {
                    tokio::select! {
                        () = operation_worker.wake.notified() => {
                            set_subscription_terminal(&terminal_worker, RedisError::ClientCancelled);
                            let mut command = redis::cmd("UNSUBSCRIBE");
                            command.arg(channel_worker.as_slice());
                            let unsubscribe = command.query_async::<()>(&mut connection);
                            let _ = tokio::time::timeout(teardown_timeout, unsubscribe).await;
                        }
                        () = disconnect.notified() => {}
                    }
                    registry.release(&operation_worker);
                    long_operation_worker.store(false, Ordering::Release);
                    wake_worker.notify_one();
                })
            }
        };
        claim.commit();
        Ok(RedisSubscriptionStream {
            messages: messages_rx,
            terminal,
            terminal_wake,
            worker,
            limits: options.limits,
            max_cell_bytes: options.max_cell_bytes,
            operation,
        })
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

    pub async fn apply_reviewed_ttl_mutation(
        &self,
        authorized: AuthorizedMutationPlan,
    ) -> Result<RedisTtlMutationOutcome, RedisError> {
        let plan = authorized.plan();
        let (logical_database, key) = match plan.target() {
            MutationTarget::RedisKey {
                logical_database,
                key,
            } => (*logical_database, key),
            _ => return Err(RedisError::InvalidMutation),
        };
        if logical_database != self.logical_database {
            return Err(RedisError::LogicalDatabaseMismatch);
        }
        if plan.changes().len() != 1
            || plan.changes().iter().any(|change| {
                !matches!(
                    change,
                    MutationChange::RedisSetExpiration(
                        RedisExpiration::Persist | RedisExpiration::ExpireAfterMillis(_)
                    )
                )
            })
        {
            return Err(RedisError::InvalidMutation);
        }

        let mut connection = self.connection.clone();
        let applied: i64 = match &plan.changes()[0] {
            MutationChange::RedisSetExpiration(RedisExpiration::Persist) => redis::cmd("PERSIST")
                .arg(key.as_slice())
                .query_async(&mut connection)
                .await
                .map_err(map_mutation_error)?,
            MutationChange::RedisSetExpiration(RedisExpiration::ExpireAfterMillis(
                milliseconds,
            )) => redis::cmd("PEXPIRE")
                .arg(key.as_slice())
                .arg(*milliseconds)
                .query_async(&mut connection)
                .await
                .map_err(map_mutation_error)?,
            _ => unreachable!("all mutation changes were validated before dispatch"),
        };
        let application = match applied {
            0 => RedisTtlApplication::NotApplied,
            1 => RedisTtlApplication::Applied,
            _ => return Err(RedisError::Protocol),
        };
        Ok(RedisTtlMutationOutcome {
            mutation_id: plan.mutation_id(),
            review_token_id: authorized.token_id(),
            application,
        })
    }
}

fn map_connect_error(error: redis::RedisError) -> RedisError {
    if error.is_timeout() {
        RedisError::Timeout
    } else if is_authentication_error(&error) {
        RedisError::Authentication
    } else {
        RedisError::Connect
    }
}

fn map_client_build_error(error: redis::RedisError) -> RedisError {
    if error.kind() == ErrorKind::InvalidClientConfig {
        RedisError::TlsConfiguration
    } else {
        map_connect_error(error)
    }
}

fn validate_security(
    tls_mode: RedisTlsMode,
    security: RedisConnectionSecurity<'_>,
) -> Result<(), RedisError> {
    if (tls_mode == RedisTlsMode::Disable) == security.tls.is_some() {
        return Err(RedisError::TlsConfiguration);
    }
    if let Some(credentials) = security.credentials
        && (credentials.password.is_empty()
            || credentials.password.len() > MAX_REDIS_CREDENTIAL_BYTES
            || credentials.username.is_some_and(|username| {
                username.is_empty() || username.len() > MAX_REDIS_CREDENTIAL_BYTES
            }))
    {
        return Err(RedisError::Authentication);
    }
    if let Some(tls) = security.tls
        && (matches!(
            tls.trust_roots,
            RedisTrustRoots::Custom(certificates)
                if certificates.is_empty()
                    || certificates.len() > MAX_REDIS_TLS_MATERIAL_BYTES
                    || !valid_custom_roots(certificates)
        ) || tls.client_identity.is_some_and(|identity| {
            identity.certificate_chain_pem.is_empty()
                || identity.certificate_chain_pem.len() > MAX_REDIS_TLS_MATERIAL_BYTES
                || identity.private_key_pem.is_empty()
                || identity.private_key_pem.len() > MAX_REDIS_TLS_MATERIAL_BYTES
                || CertificateDer::pem_slice_iter(identity.certificate_chain_pem)
                    .next()
                    .is_none_or(|certificate| certificate.is_err())
                || PrivateKeyDer::from_pem_slice(identity.private_key_pem).is_err()
        }))
    {
        return Err(RedisError::TlsConfiguration);
    }
    Ok(())
}

fn valid_custom_roots(certificates: &[u8]) -> bool {
    let mut roots = RootCertStore::empty();
    let mut count = 0_usize;
    for certificate in CertificateDer::pem_slice_iter(certificates) {
        let Ok(certificate) = certificate else {
            return false;
        };
        if roots.add(certificate).is_err() {
            return false;
        }
        count += 1;
    }
    count > 0
}

fn map_command_error(error: redis::RedisError) -> RedisError {
    if error.is_timeout() {
        RedisError::Timeout
    } else if is_authentication_error(&error) {
        RedisError::Authentication
    } else if error.is_connection_dropped() || error.is_io_error() {
        RedisError::Connection
    } else {
        RedisError::Command
    }
}

fn map_mutation_error(error: redis::RedisError) -> RedisError {
    if error.is_timeout() || error.is_connection_dropped() || error.is_io_error() {
        RedisError::WriteOutcomeUnknown
    } else {
        map_command_error(error)
    }
}

fn is_authentication_error(error: &redis::RedisError) -> bool {
    error.kind() == ErrorKind::AuthenticationFailed
        || matches!(error.code(), Some("WRONGPASS" | "NOAUTH"))
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
    long_operation_active: Arc<AtomicBool>,
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
        self.long_operation_active.store(false, Ordering::Release);
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
        self.long_operation_active.store(false, Ordering::Release);
    }
}

pub struct RedisSubscriptionStream {
    messages: tokio::sync::mpsc::Receiver<(Vec<u8>, Vec<u8>)>,
    terminal: Arc<Mutex<Option<RedisError>>>,
    terminal_wake: Arc<tokio::sync::Notify>,
    worker: tokio::task::JoinHandle<()>,
    limits: PageLimits,
    max_cell_bytes: u64,
    operation: Arc<RedisSubscriptionOperation>,
}

impl RedisSubscriptionStream {
    pub async fn next_page(
        &mut self,
        identity: PageIdentity,
        start_row: u64,
    ) -> Result<Option<ResultPage>, RedisError> {
        let first = loop {
            if let Ok(message) = self.messages.try_recv() {
                break message;
            }
            let terminal = {
                self.terminal
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone()
            };
            if let Some(error) = terminal {
                let _ = (&mut self.worker).await;
                return Err(error);
            }
            tokio::select! {
                message = self.messages.recv() => match message {
                    Some(message) => break message,
                    None => continue,
                },
                () = self.terminal_wake.notified() => {}
            }
        };
        let mut rows = vec![first];
        while rows.len() < self.limits.max_rows() as usize {
            match self.messages.try_recv() {
                Ok(message) => rows.push(message),
                Err(_) => break,
            }
        }
        let mut values = Vec::with_capacity(rows.len() * 2);
        let mut arena_remaining = self.limits.max_arena_bytes();
        for (channel, payload) in rows {
            let channel = bounded_binary(&channel, self.max_cell_bytes.min(arena_remaining))?;
            arena_remaining = arena_remaining.saturating_sub(channel.encoded_byte_len());
            let payload = bounded_binary(&payload, self.max_cell_bytes.min(arena_remaining))?;
            arena_remaining = arena_remaining.saturating_sub(payload.encoded_byte_len());
            values.extend([channel, payload]);
        }
        let mut warnings = PageWarnings::none();
        if values.iter().any(OwnedValue::is_truncated) {
            warnings = warnings.with(PageWarning::ByteLimitReached);
        }
        if values.len() / 2 == self.limits.max_rows() as usize {
            warnings = warnings.with(PageWarning::RowLimitReached);
        }
        ResultPage::from_row_major(
            identity,
            start_row,
            RowTotal::Unknown,
            PageFacts::new(PageDelivery::Partial, warnings),
            redis_binary_columns(&["channel", "payload"])?,
            values,
            self.limits,
        )
        .map(Some)
        .map_err(RedisError::Page)
    }
}

impl Drop for RedisSubscriptionStream {
    fn drop(&mut self) {
        self.operation
            .cancel_requested
            .store(true, Ordering::Release);
        self.operation.wake.notify_one();
    }
}

fn redis_binary_columns(names: &[&str]) -> Result<Vec<ColumnMetadata>, RedisError> {
    names
        .iter()
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
        .collect()
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
    fn security_debug_redacts_credentials_and_private_material() {
        let security = RedisConnectionSecurity::new()
            .with_credentials(RedisCredentials::new(
                Some("SECRET_USERNAME"),
                "SECRET_PASSWORD",
            ))
            .with_tls(
                RedisTlsMaterial::custom_roots(b"SECRET_CA").with_client_identity(
                    RedisClientIdentity::new(b"SECRET_CERTIFICATE", b"SECRET_PRIVATE_KEY"),
                ),
            );
        let debug = format!("{security:?}");
        for secret in [
            "SECRET_USERNAME",
            "SECRET_PASSWORD",
            "SECRET_CA",
            "SECRET_CERTIFICATE",
            "SECRET_PRIVATE_KEY",
        ] {
            assert!(!debug.contains(secret));
        }
        assert!(debug.contains("has_username: true"));
        assert!(debug.contains("has_client_identity: true"));
    }

    #[test]
    fn command_path_preserves_authentication_failure_class() {
        let error = redis::RedisError::from((
            ErrorKind::AuthenticationFailed,
            "synthetic authentication failure",
        ));
        assert_eq!(map_command_error(error), RedisError::Authentication);
    }

    #[test]
    fn security_rejects_invalid_credentials_tls_material_and_downgrade() {
        let oversized_tls = vec![0_u8; MAX_REDIS_TLS_MATERIAL_BYTES + 1];
        let oversized_credential = "x".repeat(MAX_REDIS_CREDENTIAL_BYTES + 1);
        let maximum_credential = "x".repeat(MAX_REDIS_CREDENTIAL_BYTES);
        let key = rcgen::KeyPair::generate().unwrap();
        let certificate = rcgen::CertificateParams::new(vec!["client.test".to_owned()])
            .unwrap()
            .self_signed(&key)
            .unwrap();
        let certificate_pem = certificate.pem();
        let key_pem = key.serialize_pem();
        assert_eq!(
            validate_security(
                RedisTlsMode::Disable,
                RedisConnectionSecurity::new().with_tls(RedisTlsMaterial::custom_roots(b"ca")),
            ),
            Err(RedisError::TlsConfiguration)
        );
        assert_eq!(
            validate_security(RedisTlsMode::Require, RedisConnectionSecurity::new(),),
            Err(RedisError::TlsConfiguration)
        );
        for credentials in [
            RedisCredentials::new(None, ""),
            RedisCredentials::new(None, &oversized_credential),
            RedisCredentials::new(Some(""), "password"),
            RedisCredentials::new(Some(&oversized_credential), "password"),
        ] {
            assert_eq!(
                validate_security(
                    RedisTlsMode::Require,
                    RedisConnectionSecurity::new()
                        .with_credentials(credentials)
                        .with_tls(RedisTlsMaterial::platform_roots()),
                ),
                Err(RedisError::Authentication)
            );
        }
        assert_eq!(
            validate_security(
                RedisTlsMode::Require,
                RedisConnectionSecurity::new()
                    .with_credentials(RedisCredentials::new(
                        Some(&maximum_credential),
                        &maximum_credential,
                    ))
                    .with_tls(RedisTlsMaterial::platform_roots()),
            ),
            Ok(())
        );
        for tls in [
            RedisTlsMaterial::custom_roots(b""),
            RedisTlsMaterial::custom_roots(&oversized_tls),
            RedisTlsMaterial::custom_roots(certificate_pem.as_bytes())
                .with_client_identity(RedisClientIdentity::new(b"", b"key")),
            RedisTlsMaterial::custom_roots(certificate_pem.as_bytes())
                .with_client_identity(RedisClientIdentity::new(b"cert", b"")),
            RedisTlsMaterial::custom_roots(certificate_pem.as_bytes())
                .with_client_identity(RedisClientIdentity::new(&oversized_tls, b"key")),
            RedisTlsMaterial::custom_roots(certificate_pem.as_bytes())
                .with_client_identity(RedisClientIdentity::new(b"cert", &oversized_tls)),
        ] {
            assert_eq!(
                validate_security(
                    RedisTlsMode::Require,
                    RedisConnectionSecurity::new().with_tls(tls),
                ),
                Err(RedisError::TlsConfiguration)
            );
        }
        assert_eq!(
            validate_security(
                RedisTlsMode::Require,
                RedisConnectionSecurity::new().with_tls(
                    RedisTlsMaterial::platform_roots().with_client_identity(
                        RedisClientIdentity::new(certificate_pem.as_bytes(), key_pem.as_bytes(),)
                    ),
                ),
            ),
            Ok(())
        );
    }

    #[tokio::test]
    async fn malformed_custom_root_fails_before_network_io() {
        let config = RedisConnectConfig::new(
            BoundedText::copy_from_str("127.0.0.1", ByteLimit::new(64)).unwrap(),
            1,
            0,
            RedisProtocol::Resp3,
            RedisTlsMode::Require,
        );
        assert!(matches!(
            RedisSession::connect(
                &config,
                RedisConnectionSecurity::new()
                    .with_tls(RedisTlsMaterial::custom_roots(b"not a PEM certificate")),
            )
            .await,
            Err(RedisError::TlsConfiguration)
        ));
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
