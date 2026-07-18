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
    AuthorizedMutationPlan, BoundedBytes, BoundedText, ByteLimit, CatalogChildrenState,
    CatalogNodeKind, ColumnMetadata, Engine, EngineType, MutationChange, MutationId,
    MutationTarget, OwnedValue, PageDelivery, PageFacts, PageIdentity, PageLimits,
    PageValidationError, PageWarning, PageWarnings, RedisExpiration, RedisTimeToLive, ResultPage,
    ReviewTokenId, RowTotal, Truncation,
};

use crate::{
    CatalogExactness, CatalogRequest, CatalogSubtree, REDIS_DEFAULT_LOGICAL_DATABASES,
    ServerDescribe, catalog::catalog_name_list,
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
    tls_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedisTtlApplication {
    Applied,
    NotApplied,
}

/// Bounded INFO overview projection (sample-timed fields only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisInfoSnapshot {
    pub fields: Vec<(String, String)>,
    pub sampled_at_ms: u64,
}

/// One stream entry for read-only stream key views.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisStreamEntry {
    pub id: String,
    /// Flattened field/value pairs as bounded display strings (even length).
    pub fields: Vec<String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedisSubscriptionKind {
    Channel,
    Pattern,
}

struct RedisSubscriptionMessage {
    pattern: Option<RedisSubscriptionField>,
    channel: RedisSubscriptionField,
    payload: RedisSubscriptionField,
}

enum RedisSubscriptionItem {
    Message(RedisSubscriptionMessage),
    DeliveryDiscontinuity,
}

struct RedisResp3SubscriptionContext {
    client: Client,
    selector: BoundedBytes,
    kind: RedisSubscriptionKind,
    policy: RedisRuntimePolicy,
    max_field_bytes: u64,
    operation: Arc<RedisSubscriptionOperation>,
    messages: tokio::sync::mpsc::Sender<RedisSubscriptionItem>,
    terminal: Arc<Mutex<Option<RedisError>>>,
    terminal_wake: Arc<tokio::sync::Notify>,
    tls_required: bool,
}

struct RedisResp3SubscriptionConnection {
    connection: redis::aio::MultiplexedConnection,
    disconnect: Arc<tokio::sync::Notify>,
    active: Arc<AtomicBool>,
    discontinuity_pending: Arc<AtomicBool>,
}

struct RedisResp3GenerationGuard {
    active: Arc<AtomicBool>,
    armed: bool,
}

impl RedisResp3GenerationGuard {
    fn new() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(true)),
            armed: true,
        }
    }

    fn active(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.active)
    }

    fn commit(mut self) -> Arc<AtomicBool> {
        self.armed = false;
        Arc::clone(&self.active)
    }
}

impl Drop for RedisResp3GenerationGuard {
    fn drop(&mut self) {
        if self.armed {
            self.active.store(false, Ordering::Release);
        }
    }
}

struct RedisSubscriptionField {
    bytes: BoundedBytes,
    original_byte_len: u64,
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

async fn await_subscription_phase<T, F>(
    operation: &RedisSubscriptionOperation,
    timeout: Duration,
    future: F,
) -> Result<T, RedisError>
where
    F: Future<Output = Result<T, RedisError>>,
{
    tokio::time::timeout(timeout, await_subscription_setup(operation, future))
        .await
        .map_err(|_| RedisError::Timeout)?
}

async fn await_subscription_connect_phase<T, F>(
    operation: &RedisSubscriptionOperation,
    timeout: Duration,
    tls_required: bool,
    future: F,
) -> Result<T, RedisError>
where
    F: Future<Output = Result<T, RedisError>>,
{
    match await_subscription_phase(operation, timeout, future).await {
        Err(RedisError::Timeout) if tls_required => Err(RedisError::Connect),
        result => result,
    }
}

async fn connect_resp2_subscription_with_retry(
    client: &Client,
    selector: &BoundedBytes,
    kind: RedisSubscriptionKind,
    policy: RedisRuntimePolicy,
    operation: &RedisSubscriptionOperation,
    tls_required: bool,
) -> Result<redis::aio::PubSub, RedisError> {
    let mut last_error = RedisError::Connection;
    for attempt in 0..policy.reconnect_attempts() {
        if attempt > 0 {
            await_subscription_setup(operation, async {
                tokio::time::sleep(subscription_retry_delay(policy, attempt - 1)).await;
                Ok(())
            })
            .await?;
        }
        let result = match await_subscription_connect_phase(
            operation,
            policy.connection_timeout(),
            tls_required,
            async { client.get_async_pubsub().await.map_err(map_connect_error) },
        )
        .await
        {
            Ok(mut pubsub) => {
                match await_subscription_phase(operation, policy.response_timeout(), async {
                    match kind {
                        RedisSubscriptionKind::Channel => {
                            pubsub.subscribe(selector.as_slice()).await
                        }
                        RedisSubscriptionKind::Pattern => {
                            pubsub.psubscribe(selector.as_slice()).await
                        }
                    }
                    .map_err(map_command_error)
                })
                .await
                {
                    Ok(()) => Ok(pubsub),
                    Err(error) => Err(error),
                }
            }
            Err(error) => Err(error),
        };
        match result {
            Ok(pubsub) => return Ok(pubsub),
            Err(error @ (RedisError::Authentication | RedisError::Protocol)) => return Err(error),
            Err(RedisError::ClientCancelled) => return Err(RedisError::ClientCancelled),
            Err(error) => last_error = error,
        }
    }
    Err(last_error)
}

fn subscription_retry_delay(policy: RedisRuntimePolicy, attempt: usize) -> Duration {
    let exponent = u32::try_from(attempt).unwrap_or(u32::MAX).min(31);
    policy
        .reconnect_min_delay()
        .saturating_mul(1_u32 << exponent)
        .min(policy.reconnect_max_delay())
}

fn send_subscription_discontinuity(
    pending: &AtomicBool,
    messages: &tokio::sync::mpsc::Sender<RedisSubscriptionItem>,
) -> Result<(), RedisError> {
    if pending.swap(false, Ordering::AcqRel) {
        messages
            .try_send(RedisSubscriptionItem::DeliveryDiscontinuity)
            .map_err(|error| match error {
                tokio::sync::mpsc::error::TrySendError::Full(_) => RedisError::SubscriptionOverflow,
                tokio::sync::mpsc::error::TrySendError::Closed(_) => RedisError::ClientCancelled,
            })?;
    }
    Ok(())
}

async fn connect_resp3_subscription(
    context: &RedisResp3SubscriptionContext,
    announce_discontinuity: bool,
) -> Result<RedisResp3SubscriptionConnection, RedisError> {
    let generation = RedisResp3GenerationGuard::new();
    let disconnect = Arc::new(tokio::sync::Notify::new());
    let disconnect_sender = Arc::clone(&disconnect);
    let active_sender = generation.active();
    let discontinuity_pending = Arc::new(AtomicBool::new(announce_discontinuity));
    let discontinuity_sender = Arc::clone(&discontinuity_pending);
    let terminal = Arc::clone(&context.terminal);
    let terminal_wake = Arc::clone(&context.terminal_wake);
    let messages = context.messages.clone();
    let kind = context.kind;
    let max_field_bytes = context.max_field_bytes;
    let push_sender = move |push: PushInfo| -> Result<(), ()> {
        if !active_sender.load(Ordering::Acquire) {
            return Ok(());
        }
        match &push.kind {
            PushKind::Message | PushKind::PMessage => {
                let Some(message) = redis::Msg::from_push_info(push) else {
                    set_subscription_terminal(&terminal, RedisError::Protocol);
                    disconnect_sender.notify_one();
                    terminal_wake.notify_one();
                    return Err(());
                };
                let decoded = (
                    message.get_pattern::<Option<Vec<u8>>>(),
                    message.get_channel::<Vec<u8>>(),
                    message.get_payload::<Vec<u8>>(),
                );
                let (Ok(pattern), Ok(channel), Ok(payload)) = decoded else {
                    set_subscription_terminal(&terminal, RedisError::Protocol);
                    disconnect_sender.notify_one();
                    terminal_wake.notify_one();
                    return Err(());
                };
                if pattern.is_some() != matches!(kind, RedisSubscriptionKind::Pattern) {
                    set_subscription_terminal(&terminal, RedisError::Protocol);
                    disconnect_sender.notify_one();
                    terminal_wake.notify_one();
                    return Err(());
                }
                send_subscription_discontinuity(&discontinuity_sender, &messages)
                    .and_then(|()| {
                        bounded_subscription_message(pattern, channel, payload, max_field_bytes)
                    })
                    .and_then(|message| {
                        messages
                            .try_send(RedisSubscriptionItem::Message(message))
                            .map_err(|error| match error {
                                tokio::sync::mpsc::error::TrySendError::Full(_) => {
                                    RedisError::SubscriptionOverflow
                                }
                                tokio::sync::mpsc::error::TrySendError::Closed(_) => {
                                    RedisError::ClientCancelled
                                }
                            })
                    })
                    .map_err(|error| {
                        set_subscription_terminal(&terminal, error);
                        disconnect_sender.notify_one();
                        terminal_wake.notify_one();
                    })
            }
            PushKind::Disconnection => {
                disconnect_sender.notify_one();
                Ok(())
            }
            _ => Ok(()),
        }
    };
    let config = context
        .policy
        .blocking_config()
        .set_push_sender(push_sender);
    let mut connection = await_subscription_connect_phase(
        &context.operation,
        context.policy.connection_timeout(),
        context.tls_required,
        async {
            context
                .client
                .get_multiplexed_async_connection_with_config(&config)
                .await
                .map_err(map_connect_error)
        },
    )
    .await?;
    await_subscription_phase(
        &context.operation,
        context.policy.response_timeout(),
        async {
            redis::cmd(match context.kind {
                RedisSubscriptionKind::Channel => "SUBSCRIBE",
                RedisSubscriptionKind::Pattern => "PSUBSCRIBE",
            })
            .arg(context.selector.as_slice())
            .query_async::<()>(&mut connection)
            .await
            .map_err(map_command_error)
        },
    )
    .await?;
    Ok(RedisResp3SubscriptionConnection {
        connection,
        disconnect,
        active: generation.commit(),
        discontinuity_pending,
    })
}

async fn connect_resp3_subscription_with_retry(
    context: &RedisResp3SubscriptionContext,
    announce_discontinuity: bool,
) -> Result<RedisResp3SubscriptionConnection, RedisError> {
    let mut last_error = RedisError::Connection;
    for attempt in 0..context.policy.reconnect_attempts() {
        if attempt > 0 {
            await_subscription_setup(&context.operation, async {
                tokio::time::sleep(subscription_retry_delay(context.policy, attempt - 1)).await;
                Ok(())
            })
            .await?;
        }
        match connect_resp3_subscription(context, announce_discontinuity).await {
            Ok(connection) => return Ok(connection),
            Err(error @ (RedisError::Authentication | RedisError::Protocol)) => return Err(error),
            Err(RedisError::ClientCancelled) => return Err(RedisError::ClientCancelled),
            Err(error) => last_error = error,
        }
    }
    Err(last_error)
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
            tls_required: matches!(config.tls, RedisTlsMode::Require),
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

    pub async fn health_check(&self) -> Result<(), RedisError> {
        let mut connection = self.connection.clone();
        redis::cmd("PING")
            .query_async::<String>(&mut connection)
            .await
            .map(|_| ())
            .map_err(|_| RedisError::Connection)
    }

    pub async fn describe_server(&self) -> Result<ServerDescribe, RedisError> {
        let started = std::time::Instant::now();
        let mut connection = self.connection.clone();
        let info: String = redis::cmd("INFO")
            .arg("server")
            .query_async(&mut connection)
            .await
            .map_err(|_| RedisError::Command)?;
        let version = info
            .lines()
            .find_map(|line| line.strip_prefix("redis_version:"))
            .unwrap_or("redis")
            .chars()
            .take(64)
            .collect::<String>();
        Ok(ServerDescribe::new(
            Engine::Redis,
            format!("Redis {version}"),
            u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        ))
    }

    pub async fn list_catalog(
        &self,
        request: CatalogRequest,
    ) -> Result<CatalogSubtree, RedisError> {
        match request {
            CatalogRequest::RedisLogicalDatabases { limits } => {
                self.catalog_logical_databases(limits.max_rows()).await
            }
            _ => Err(RedisError::Command),
        }
    }

    async fn catalog_logical_databases(&self, limit: u32) -> Result<CatalogSubtree, RedisError> {
        if limit == 0 {
            return Err(RedisError::InvalidLimits);
        }
        let mut control = self.control.clone();
        let (count, exactness) = match redis::cmd("CONFIG")
            .arg("GET")
            .arg("databases")
            .query_async::<Vec<String>>(&mut control)
            .await
        {
            Ok(pairs) => {
                let value = pairs
                    .windows(2)
                    .find(|window| window[0].eq_ignore_ascii_case("databases"))
                    .map(|window| window[1].as_str())
                    .or_else(|| pairs.get(1).map(String::as_str))
                    .unwrap_or("16");
                let parsed = value
                    .parse::<u32>()
                    .unwrap_or(REDIS_DEFAULT_LOGICAL_DATABASES)
                    .max(1);
                (parsed, CatalogExactness::Exact)
            }
            Err(_) => (
                REDIS_DEFAULT_LOGICAL_DATABASES,
                CatalogExactness::DefaultAssumed,
            ),
        };
        let take = count.min(limit);
        let truncated = count > limit;
        let names = (0..take).map(|index| format!("db{index}"));
        let subtree = catalog_name_list(
            Engine::Redis,
            names,
            CatalogNodeKind::RedisLogicalDatabase,
            CatalogChildrenState::Unrequested,
            limit,
        );
        Ok(CatalogSubtree::new(
            Engine::Redis,
            subtree.into_nodes(),
            !truncated,
            if truncated {
                CatalogExactness::Truncated
            } else {
                exactness
            },
        ))
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
        self.start_subscription(channel, RedisSubscriptionKind::Channel, options)
            .await
    }

    pub async fn psubscribe(
        &self,
        pattern: BoundedBytes,
        options: RedisSubscriptionOptions,
    ) -> Result<RedisSubscriptionStream, RedisError> {
        self.start_subscription(pattern, RedisSubscriptionKind::Pattern, options)
            .await
    }

    async fn start_subscription(
        &self,
        selector: BoundedBytes,
        kind: RedisSubscriptionKind,
        options: RedisSubscriptionOptions,
    ) -> Result<RedisSubscriptionStream, RedisError> {
        let required_columns = match kind {
            RedisSubscriptionKind::Channel => 2,
            RedisSubscriptionKind::Pattern => 3,
        };
        if selector.is_empty()
            || u64::try_from(selector.len()).unwrap_or(u64::MAX) > options.max_cell_bytes
            || options.limits.max_rows() == 0
            || options.limits.max_columns() < required_columns
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
                let pubsub = connect_resp2_subscription_with_retry(
                    &self.client,
                    &selector,
                    kind,
                    self.runtime_policy,
                    &operation,
                    self.tls_required,
                )
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
                let selector_worker = selector.clone();
                let teardown_timeout = self.runtime_policy.response_timeout();
                let reconnect_policy = self.runtime_policy;
                let client_worker = self.client.clone();
                let tls_required = self.tls_required;
                tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            () = operation_worker.wake.notified() => {
                                set_subscription_terminal(&terminal_worker, RedisError::ClientCancelled);
                                let unsubscribe = async {
                                    match kind {
                                        RedisSubscriptionKind::Channel => sink.unsubscribe(selector_worker.as_slice()).await,
                                        RedisSubscriptionKind::Pattern => sink.punsubscribe(selector_worker.as_slice()).await,
                                    }
                                };
                                let _ = tokio::time::timeout(teardown_timeout, unsubscribe).await;
                                break;
                            }
                            message = stream.next() => match message {
                                Some(message) => {
                                    let pattern = message.get_pattern::<Option<Vec<u8>>>();
                                    let item = (pattern, message.get_channel::<Vec<u8>>(), message.get_payload::<Vec<u8>>());
                                    match item {
                                        (Ok(pattern), Ok(channel), Ok(payload)) if pattern.is_some() == matches!(kind, RedisSubscriptionKind::Pattern) => match bounded_subscription_message(pattern, channel, payload, options.max_cell_bytes).and_then(|message| messages_tx.try_send(RedisSubscriptionItem::Message(message)).map_err(|error| match error {
                                            tokio::sync::mpsc::error::TrySendError::Full(_) => RedisError::SubscriptionOverflow,
                                            tokio::sync::mpsc::error::TrySendError::Closed(_) => RedisError::ClientCancelled,
                                        })) {
                                            Ok(()) => {}
                                            Err(error) => {
                                                set_subscription_terminal(&terminal_worker, error);
                                                let unsubscribe = async {
                                                    match kind {
                                                        RedisSubscriptionKind::Channel => sink.unsubscribe(selector_worker.as_slice()).await,
                                                        RedisSubscriptionKind::Pattern => sink.punsubscribe(selector_worker.as_slice()).await,
                                                    }
                                                };
                                                let _ = tokio::time::timeout(teardown_timeout, unsubscribe).await;
                                                break;
                                            }
                                        },
                                        _ => {
                                            set_subscription_terminal(&terminal_worker, RedisError::Protocol);
                                            break;
                                        }
                                    }
                                }
                                None => {
                                    match connect_resp2_subscription_with_retry(
                                        &client_worker,
                                        &selector_worker,
                                        kind,
                                        reconnect_policy,
                                        &operation_worker,
                                        tls_required,
                                    ).await {
                                        Ok(pubsub) => {
                                            let replacement = pubsub.split();
                                            sink = replacement.0;
                                            stream = replacement.1;
                                            if messages_tx.try_send(RedisSubscriptionItem::DeliveryDiscontinuity).is_err() {
                                                set_subscription_terminal(&terminal_worker, RedisError::SubscriptionOverflow);
                                                break;
                                            }
                                        }
                                        Err(error) => {
                                            set_subscription_terminal(&terminal_worker, error);
                                            break;
                                        }
                                    }
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
                let reconnect_context = RedisResp3SubscriptionContext {
                    client: self.client.clone(),
                    selector: selector.clone(),
                    kind,
                    policy: self.runtime_policy,
                    max_field_bytes: options.max_cell_bytes,
                    operation: Arc::clone(&operation),
                    messages: messages_tx.clone(),
                    terminal: Arc::clone(&terminal),
                    terminal_wake: Arc::clone(&terminal_wake),
                    tls_required: self.tls_required,
                };
                let initial =
                    connect_resp3_subscription_with_retry(&reconnect_context, false).await?;
                let mut connection = initial.connection;
                let mut disconnect = initial.disconnect;
                let mut active = initial.active;
                operation.started.store(true, Ordering::Release);
                if operation.cancel_requested.load(Ordering::Acquire) {
                    operation.wake.notify_one();
                }
                let operation_worker = Arc::clone(&operation);
                let terminal_worker = Arc::clone(&terminal);
                let wake_worker = Arc::clone(&terminal_wake);
                let long_operation_worker = Arc::clone(&self.long_operation_active);
                let selector_worker = selector.clone();
                let teardown_timeout = self.runtime_policy.response_timeout();
                tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            () = operation_worker.wake.notified() => {
                                active.store(false, Ordering::Release);
                                set_subscription_terminal(&terminal_worker, RedisError::ClientCancelled);
                                let mut command = redis::cmd(match kind {
                                    RedisSubscriptionKind::Channel => "UNSUBSCRIBE",
                                    RedisSubscriptionKind::Pattern => "PUNSUBSCRIBE",
                                });
                                command.arg(selector_worker.as_slice());
                                let unsubscribe = command.query_async::<()>(&mut connection);
                                let _ = tokio::time::timeout(teardown_timeout, unsubscribe).await;
                                break;
                            }
                            () = disconnect.notified() => {
                                active.store(false, Ordering::Release);
                                if terminal_worker.lock().unwrap_or_else(|e| e.into_inner()).is_some() {
                                    break;
                                }
                                match connect_resp3_subscription_with_retry(&reconnect_context, true).await {
                                    Ok(replacement) => {
                                        connection = replacement.connection;
                                        disconnect = replacement.disconnect;
                                        active = replacement.active;
                                        if let Err(error) = send_subscription_discontinuity(
                                            &replacement.discontinuity_pending,
                                            &messages_tx,
                                        ) {
                                            active.store(false, Ordering::Release);
                                            set_subscription_terminal(&terminal_worker, error);
                                            break;
                                        }
                                    }
                                    Err(error) => {
                                        set_subscription_terminal(&terminal_worker, error);
                                        break;
                                    }
                                }
                            }
                        }
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
            operation,
            kind,
            pending_discontinuity: false,
        })
    }

    pub fn scan_keys(
        &self,
        limits: PageLimits,
        max_cell_bytes: u64,
        scan_count: u32,
        max_scan_rounds: u32,
        match_pattern: Option<BoundedBytes>,
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
        // Empty / "*" → no MATCH clause (full namespace SCAN with COUNT).
        let match_pattern = match_pattern.and_then(|p| {
            let bytes = p.as_slice();
            if bytes.is_empty() || bytes == b"*" {
                None
            } else {
                Some(p)
            }
        });
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
            match_pattern,
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

    /// Redis TYPE for a key → catalog-aligned kind.
    pub async fn key_type(
        &self,
        key: &BoundedBytes,
    ) -> Result<tablerock_core::RedisKeyKind, RedisError> {
        let mut connection = self.connection.clone();
        let kind: String = redis::cmd("TYPE")
            .arg(key.as_slice())
            .query_async(&mut connection)
            .await
            .map_err(map_command_error)?;
        Ok(match kind.as_str() {
            "string" => tablerock_core::RedisKeyKind::String,
            "hash" => tablerock_core::RedisKeyKind::Hash,
            "list" => tablerock_core::RedisKeyKind::List,
            "set" => tablerock_core::RedisKeyKind::Set,
            "zset" => tablerock_core::RedisKeyKind::SortedSet,
            "stream" => tablerock_core::RedisKeyKind::Stream,
            "none" => tablerock_core::RedisKeyKind::Unknown,
            _ => tablerock_core::RedisKeyKind::Unknown,
        })
    }

    /// Bounded LRANGE for list key views (`start`/`stop` inclusive, Redis rules).
    pub async fn list_range(
        &self,
        key: &BoundedBytes,
        start: i64,
        stop: i64,
        max_entries: u32,
        max_cell_bytes: u64,
    ) -> Result<Vec<OwnedValue>, RedisError> {
        if max_entries == 0 || max_cell_bytes == 0 {
            return Err(RedisError::InvalidLimits);
        }
        let mut connection = self.connection.clone();
        let values: Vec<Vec<u8>> = redis::cmd("LRANGE")
            .arg(key.as_slice())
            .arg(start)
            .arg(stop)
            .query_async(&mut connection)
            .await
            .map_err(map_command_error)?;
        let mut out = Vec::new();
        for value in values.into_iter().take(max_entries as usize) {
            out.push(bounded_binary(&value, max_cell_bytes)?);
        }
        Ok(out)
    }

    /// Bounded XRANGE for stream read-only views.
    ///
    /// Returns (id, field_value_pairs_as_flat_bytes_display) rows. Field values
    /// are truncated per `max_cell_bytes`.
    pub async fn stream_range(
        &self,
        key: &BoundedBytes,
        start_id: &str,
        end_id: &str,
        count: u32,
        max_cell_bytes: u64,
    ) -> Result<Vec<RedisStreamEntry>, RedisError> {
        if count == 0 || max_cell_bytes == 0 || start_id.is_empty() || end_id.is_empty() {
            return Err(RedisError::InvalidLimits);
        }
        let mut connection = self.connection.clone();
        // XRANGE key start end COUNT n
        let raw: redis::Value = redis::cmd("XRANGE")
            .arg(key.as_slice())
            .arg(start_id)
            .arg(end_id)
            .arg("COUNT")
            .arg(count)
            .query_async(&mut connection)
            .await
            .map_err(map_command_error)?;
        decode_stream_entries(raw, max_cell_bytes)
    }

    /// Dispatch a pre-tokenized command. Blocking names are rejected here.
    ///
    /// `name` must already be uppercased. Unknown names are allowed (writes).
    /// The caller (TUI command classifier) must deny KEYS for browse; this path
    /// still accepts KEYS as a deliberate operator command (not auto-browse).
    pub async fn execute_command_argv(
        &self,
        name: &str,
        args: &[Vec<u8>],
    ) -> Result<redis::Value, RedisError> {
        if name.is_empty() {
            return Err(RedisError::InvalidLimits);
        }
        let upper = name.to_ascii_uppercase();
        // Shared-session deny list (matches TUI BlockingDenied).
        const BLOCKING: &[&str] = &[
            "BLPOP", "BRPOP", "BRPOPLPUSH", "BLMOVE", "BZPOPMIN", "BZPOPMAX", "BZMPOP",
            "BLMPOP", "XREAD", "XREADGROUP",
        ];
        if BLOCKING.contains(&upper.as_str()) {
            return Err(RedisError::InvalidMutation);
        }
        let mut connection = self.connection.clone();
        let mut cmd = redis::cmd(&upper);
        for arg in args {
            cmd.arg(arg.as_slice());
        }
        cmd.query_async(&mut connection)
            .await
            .map_err(map_command_error)
    }

    /// Bounded INFO snapshot as (section_or_key, value) lines with sample time.
    pub async fn server_info_snapshot(&self) -> Result<RedisInfoSnapshot, RedisError> {
        let mut connection = self.connection.clone();
        let raw: String = redis::cmd("INFO")
            .query_async(&mut connection)
            .await
            .map_err(map_command_error)?;
        let sampled_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let mut fields = Vec::new();
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once(':') {
                // Cap field count for bounded overview.
                if fields.len() >= 256 {
                    break;
                }
                fields.push((k.to_owned(), v.to_owned()));
            }
        }
        Ok(RedisInfoSnapshot {
            fields,
            sampled_at_ms,
        })
    }

    /// Sequential apply for Redis string/TTL/delete plans (non-transactional).
    pub async fn apply_authorized_mutation(
        &self,
        authorized: AuthorizedMutationPlan,
    ) -> Result<crate::MutationApplyOutcome, RedisError> {
        use crate::postgres_mutation::{
            MutationApplyOutcome, MutationChangeOutcome, MutationTransactionState,
        };
        use tablerock_core::MutationChange;

        let plan = authorized.plan();
        let MutationTarget::RedisKey {
            logical_database,
            key,
        } = plan.target()
        else {
            return Err(RedisError::InvalidMutation);
        };
        if *logical_database != self.logical_database {
            return Err(RedisError::InvalidMutation);
        }
        let mut connection = self.connection.clone();
        let mut outcomes = Vec::new();
        for (index, change) in plan.changes().iter().enumerate() {
            let outcome = match change {
                MutationChange::RedisSetString { value, expiration } => {
                    let mut cmd = redis::cmd("SET");
                    cmd.arg(key.as_slice()).arg(value.as_slice());
                    match expiration {
                        tablerock_core::RedisExpiration::Persist => {
                            // No TTL args — leaves any existing TTL? SET replaces key.
                        }
                        tablerock_core::RedisExpiration::Preserve => {
                            // Prefer SET KEEPTTL when available.
                            cmd.arg("KEEPTTL");
                        }
                        tablerock_core::RedisExpiration::ExpireAfterMillis(ms) => {
                            cmd.arg("PX").arg(*ms);
                        }
                    }
                    cmd.query_async::<()>(&mut connection)
                        .await
                        .map_err(map_command_error)?;
                    MutationChangeOutcome::Applied {
                        index,
                        rows_affected: 1,
                        returned: vec![("command".into(), "SET".into())],
                    }
                }
                MutationChange::RedisDeleteKey => {
                    let n: i64 = redis::cmd("DEL")
                        .arg(key.as_slice())
                        .query_async(&mut connection)
                        .await
                        .map_err(map_command_error)?;
                    MutationChangeOutcome::Applied {
                        index,
                        rows_affected: u64::try_from(n.max(0)).unwrap_or(0),
                        returned: vec![("command".into(), "DEL".into())],
                    }
                }
                MutationChange::RedisSetExpiration(expiration) => {
                    // Delegate semantics via existing TTL path for single-change plans.
                    let _ = expiration;
                    MutationChangeOutcome::Failed {
                        index,
                        detail: "use apply_reviewed_ttl_mutation for pure TTL changes".into(),
                    }
                }
                MutationChange::InsertRow { .. }
                | MutationChange::UpdateRow { .. }
                | MutationChange::DeleteRow { .. } => MutationChangeOutcome::Failed {
                    index,
                    detail: "relational mutation not valid on Redis session".into(),
                },
            };
            let stop = matches!(
                outcome,
                MutationChangeOutcome::Failed { .. } | MutationChangeOutcome::Conflict { .. }
            );
            outcomes.push(outcome);
            if stop {
                break;
            }
        }
        Ok(MutationApplyOutcome {
            mutation_id: plan.mutation_id(),
            review_token_id: authorized.token_id(),
            // Non-transactional sequential apply finished (no rollback claim).
            transaction: MutationTransactionState::Committed,
            changes: outcomes,
        })
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
    messages: tokio::sync::mpsc::Receiver<RedisSubscriptionItem>,
    terminal: Arc<Mutex<Option<RedisError>>>,
    terminal_wake: Arc<tokio::sync::Notify>,
    worker: tokio::task::JoinHandle<()>,
    limits: PageLimits,
    operation: Arc<RedisSubscriptionOperation>,
    kind: RedisSubscriptionKind,
    pending_discontinuity: bool,
}

impl RedisSubscriptionStream {
    pub async fn next_page(
        &mut self,
        identity: PageIdentity,
        start_row: u64,
    ) -> Result<Option<ResultPage>, RedisError> {
        if std::mem::take(&mut self.pending_discontinuity) {
            return self.discontinuity_page(identity, start_row).map(Some);
        }
        let first = loop {
            if let Ok(item) = self.messages.try_recv() {
                break item;
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
                    Some(item) => break item,
                    None => continue,
                },
                () = self.terminal_wake.notified() => {}
            }
        };
        let RedisSubscriptionItem::Message(first) = first else {
            return self.discontinuity_page(identity, start_row).map(Some);
        };
        let mut rows = vec![first];
        while rows.len() < self.limits.max_rows() as usize {
            match self.messages.try_recv() {
                Ok(RedisSubscriptionItem::Message(message)) => rows.push(message),
                Ok(RedisSubscriptionItem::DeliveryDiscontinuity) => {
                    self.pending_discontinuity = true;
                    break;
                }
                Err(_) => break,
            }
        }
        let columns = match self.kind {
            RedisSubscriptionKind::Channel => &["channel", "payload"][..],
            RedisSubscriptionKind::Pattern => &["pattern", "channel", "payload"][..],
        };
        let mut values = Vec::with_capacity(rows.len() * columns.len());
        let mut arena_remaining = self.limits.max_arena_bytes();
        for message in rows {
            if let Some(pattern) = message.pattern {
                let pattern = pattern.into_owned_value(arena_remaining)?;
                arena_remaining = arena_remaining.saturating_sub(pattern.encoded_byte_len());
                values.push(pattern);
            }
            let channel = message.channel.into_owned_value(arena_remaining)?;
            arena_remaining = arena_remaining.saturating_sub(channel.encoded_byte_len());
            let payload = message.payload.into_owned_value(arena_remaining)?;
            arena_remaining = arena_remaining.saturating_sub(payload.encoded_byte_len());
            values.extend([channel, payload]);
        }
        let mut warnings = PageWarnings::none();
        if values.iter().any(OwnedValue::is_truncated) {
            warnings = warnings.with(PageWarning::ByteLimitReached);
        }
        if values.len() / columns.len() == self.limits.max_rows() as usize {
            warnings = warnings.with(PageWarning::RowLimitReached);
        }
        ResultPage::from_row_major(
            identity,
            start_row,
            RowTotal::Unknown,
            PageFacts::new(PageDelivery::Partial, warnings),
            redis_binary_columns(columns)?,
            values,
            self.limits,
        )
        .map(Some)
        .map_err(RedisError::Page)
    }

    fn discontinuity_page(
        &self,
        identity: PageIdentity,
        start_row: u64,
    ) -> Result<ResultPage, RedisError> {
        let columns = match self.kind {
            RedisSubscriptionKind::Channel => &["channel", "payload"][..],
            RedisSubscriptionKind::Pattern => &["pattern", "channel", "payload"][..],
        };
        ResultPage::from_row_major(
            identity,
            start_row,
            RowTotal::Unknown,
            PageFacts::new(
                PageDelivery::Partial,
                PageWarnings::none().with(PageWarning::DeliveryDiscontinuity),
            ),
            redis_binary_columns(columns)?,
            Vec::new(),
            self.limits,
        )
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
    /// When set, SCAN uses MATCH (never KEYS).
    match_pattern: Option<BoundedBytes>,
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
            let (cursor, keys): (u64, Vec<Vec<u8>>) = {
                let mut cmd = redis::cmd("SCAN");
                cmd.arg(self.cursor).arg("COUNT").arg(self.scan_count);
                if let Some(pattern) = self.match_pattern.as_ref() {
                    cmd.arg("MATCH").arg(pattern.as_slice());
                }
                cmd.query_async(&mut self.connection)
                    .await
                    .map_err(map_command_error)?
            };
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

fn decode_stream_entries(
    raw: redis::Value,
    max_cell_bytes: u64,
) -> Result<Vec<RedisStreamEntry>, RedisError> {
    let entries = match raw {
        redis::Value::Array(items) | redis::Value::Set(items) => items,
        redis::Value::Nil => return Ok(Vec::new()),
        _ => return Err(RedisError::Protocol),
    };
    let mut out = Vec::new();
    for entry in entries {
        let redis::Value::Array(parts) = entry else {
            return Err(RedisError::Protocol);
        };
        if parts.len() < 2 {
            return Err(RedisError::Protocol);
        }
        let id = value_as_utf8_lossy(&parts[0]);
        let mut fields = Vec::new();
        match &parts[1] {
            redis::Value::Array(f) | redis::Value::Set(f) => {
                for item in f {
                    fields.push(value_as_utf8_lossy_bounded(item, max_cell_bytes));
                }
            }
            redis::Value::Map(pairs) => {
                for (k, v) in pairs {
                    fields.push(value_as_utf8_lossy(k));
                    fields.push(value_as_utf8_lossy_bounded(v, max_cell_bytes));
                }
            }
            _ => return Err(RedisError::Protocol),
        }
        out.push(RedisStreamEntry { id, fields });
    }
    Ok(out)
}

fn value_as_utf8_lossy(value: &redis::Value) -> String {
    match value {
        redis::Value::BulkString(b) => String::from_utf8_lossy(b).into_owned(),
        redis::Value::SimpleString(s) | redis::Value::VerbatimString { text: s, .. } => s.clone(),
        redis::Value::Int(n) => n.to_string(),
        redis::Value::Nil => "∅".into(),
        other => format!("{other:?}"),
    }
}

fn value_as_utf8_lossy_bounded(value: &redis::Value, max_cell_bytes: u64) -> String {
    match value {
        redis::Value::BulkString(b) => {
            let n = b.len().min(max_cell_bytes as usize);
            let mut s = String::from_utf8_lossy(&b[..n]).into_owned();
            if b.len() > n {
                s.push('…');
            }
            s
        }
        redis::Value::SimpleString(s) | redis::Value::VerbatimString { text: s, .. } => {
            let n = s.len().min(max_cell_bytes as usize);
            let mut out = s.chars().take(n).collect::<String>();
            if s.len() > n {
                out.push('…');
            }
            out
        }
        other => value_as_utf8_lossy(other),
    }
}

fn bounded_subscription_message(
    pattern: Option<Vec<u8>>,
    channel: Vec<u8>,
    payload: Vec<u8>,
    max_field_bytes: u64,
) -> Result<RedisSubscriptionMessage, RedisError> {
    Ok(RedisSubscriptionMessage {
        pattern: pattern
            .map(|value| RedisSubscriptionField::new(value, max_field_bytes))
            .transpose()?,
        channel: RedisSubscriptionField::new(channel, max_field_bytes)?,
        payload: RedisSubscriptionField::new(payload, max_field_bytes)?,
    })
}

impl RedisSubscriptionField {
    fn new(value: Vec<u8>, limit: u64) -> Result<Self, RedisError> {
        let original_byte_len = u64::try_from(value.len()).unwrap_or(u64::MAX);
        let stored_len = usize::try_from(limit)
            .unwrap_or(usize::MAX)
            .min(value.len());
        let bytes = BoundedBytes::copy_from_slice(&value[..stored_len], ByteLimit::new(limit))
            .map_err(|_| RedisError::Protocol)?;
        Ok(Self {
            bytes,
            original_byte_len,
        })
    }

    fn into_owned_value(self, arena_remaining: u64) -> Result<OwnedValue, RedisError> {
        let stored_len = usize::try_from(arena_remaining)
            .unwrap_or(usize::MAX)
            .min(self.bytes.len());
        let bytes = BoundedBytes::copy_from_slice(
            &self.bytes.as_slice()[..stored_len],
            ByteLimit::new(arena_remaining),
        )
        .map_err(|_| RedisError::Protocol)?;
        let truncation = if u64::try_from(stored_len).unwrap_or(u64::MAX) == self.original_byte_len
        {
            Truncation::Complete
        } else {
            Truncation::Truncated {
                original_byte_len: Some(self.original_byte_len),
            }
        };
        OwnedValue::binary(bytes, truncation).map_err(|_| RedisError::Protocol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resp3_generation_guard_deactivates_only_abandoned_attempts() {
        let abandoned = RedisResp3GenerationGuard::new();
        let abandoned_active = abandoned.active();
        drop(abandoned);
        assert!(!abandoned_active.load(Ordering::Acquire));

        let committed = RedisResp3GenerationGuard::new();
        let committed_active = committed.commit();
        assert!(committed_active.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn subscription_reconnect_attempts_time_out_against_a_blackhole() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            loop {
                let Ok((connection, _)) = listener.accept().await else {
                    break;
                };
                tokio::spawn(async move {
                    let _connection = connection;
                    tokio::time::sleep(Duration::from_secs(2)).await;
                });
            }
        });
        let policy = RedisRuntimePolicy::new(
            Duration::from_millis(40),
            Duration::from_millis(40),
            2,
            Duration::from_millis(10),
            Duration::from_millis(10),
        )
        .unwrap();
        let selector = BoundedBytes::copy_from_slice(b"blackhole", ByteLimit::new(9)).unwrap();

        for (protocol, wire_protocol) in [
            (RedisProtocol::Resp2, ProtocolVersion::RESP2),
            (RedisProtocol::Resp3, ProtocolVersion::RESP3),
        ] {
            let redis = RedisConnectionInfo::default().set_protocol(wire_protocol);
            let info = ConnectionAddr::Tcp("127.0.0.1".to_owned(), port)
                .into_connection_info()
                .unwrap()
                .set_redis_settings(redis);
            let client = Client::open(info).unwrap();
            let operation = Arc::new(RedisSubscriptionOperation::default());
            let started = tokio::time::Instant::now();
            let error = match protocol {
                RedisProtocol::Resp2 => connect_resp2_subscription_with_retry(
                    &client,
                    &selector,
                    RedisSubscriptionKind::Channel,
                    policy,
                    &operation,
                    false,
                )
                .await
                .err()
                .unwrap(),
                RedisProtocol::Resp3 => {
                    let (messages, _) = tokio::sync::mpsc::channel(2);
                    connect_resp3_subscription_with_retry(
                        &RedisResp3SubscriptionContext {
                            client,
                            selector: selector.clone(),
                            kind: RedisSubscriptionKind::Channel,
                            policy,
                            max_field_bytes: 16,
                            operation,
                            messages,
                            terminal: Arc::new(Mutex::new(None)),
                            terminal_wake: Arc::new(tokio::sync::Notify::new()),
                            tls_required: false,
                        },
                        false,
                    )
                    .await
                    .err()
                    .unwrap()
                }
            };
            assert_eq!(error, RedisError::Timeout);
            assert!(started.elapsed() < Duration::from_millis(500));
        }
        server.abort();
    }

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


#[cfg(test)]
mod scan_policy_tests {
    #[test]
    fn driver_source_never_constructs_keys_command() {
        // Static policy: key browse uses SCAN only.
        let src = include_str!("redis.rs");
        let forbidden = format!("cmd(\"{}\")", "KEYS");
        let forbidden2 = format!("cmd('{}')", "KEYS");
        for line in src.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("///") {
                continue;
            }
            // Skip this test module's own pattern builders.
            if trimmed.contains("forbidden") || trimmed.contains("format!") {
                continue;
            }
            assert!(
                !trimmed.contains(&forbidden) && !trimmed.contains(&forbidden2),
                "forbidden KEYS command construction: {trimmed}"
            );
        }
    }
}
