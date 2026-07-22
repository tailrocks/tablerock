//! Coarse synchronous facade matching the shared-client-contract bridge shape.

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, UNIX_EPOCH},
};

use tablerock_core::{
    BoundedBytes, BoundedText, ByteLimit, CatalogChildrenState, CatalogNode, CatalogNodeId,
    CatalogNodeKind, ClickHouseObjectKind, CommandBudget, CommandBudgetLimits, CommandEnvelope,
    CommandIntent, CommandScope, CopyFormat, CopyTable, DangerousPlaintext, DdlKind, DdlPlan,
    DdlTarget, Engine, EnvironmentReference, EnvironmentTag, FieldValue, IdParts,
    KeychainReference, MutationChange, MutationId, MutationPlan, MutationPlanLimits,
    MutationReviewRegistry, MutationTarget, OnePasswordReference, OperationId, OperationOutcome,
    OperationScope, OwnedValue, PageIdentity, PageKey, PageLimits, PageRequest, PageWarning,
    PlaintextAcknowledgement, PostgreSqlObjectKind, ProfileAggregate, ProfileConnectionSnapshot,
    ProfileDurability, ProfileEndpointPart, ProfileGroupName, ProfileId, ProfileIdentity,
    ProfileLimits, ProfileListFilter, ProfileListRequest, ProfileName, ProfileOrganization,
    ProfilePolicy, ProfilePreferences, ProfileProperty, ProfilePropertyBinding, ProfilePropertySet,
    ProfileSafetyMode, ProfileSearchTerm, ProfileTag, ReconnectDecision, ReconnectPreference,
    RedisKeyKind, ResultStore, ResultStoreLimits, ReviewTokenId, ReviewedRoleChangePlan, Revision,
    RoleChangeKind, RoleChangePlan, SavedFilterCondition, SavedFilterLibrary, SavedFilterPreset,
    SecretSource, SecretSourceKind, ServiceCoordinator, ServiceLimits, SessionId, ShutdownMode,
    StatementText, SupportBundle, SupportPlatform, TlsPolicy, copy_cell_from_page,
    format_copy_table, is_safe_preset_name, parse_connection_url, reconnect_decision,
    rewrite_named_params,
};
use tablerock_engine::{
    AdapterFailureClass, BrowseDialect, BrowsePlan, CatalogRequest, ClickHouseCompression,
    ClickHouseConnectConfig, ClickHouseProbeQuery, ClickHouseSession, ClickHouseTlsMode,
    DriverPageRequest, DriverRuntime, DriverSession, EngineService, EngineServiceUpdate,
    FilterOperator, KeychainReadPort, MutationApplyControl, OpCliReader, PostgresConnectConfig,
    PostgresProbeQuery, PostgresSession, PostgresTlsMode, RedisConnectConfig,
    RedisConnectionSecurity, RedisCredentials, RedisProtocol, RedisSession, RedisSubscriptionKind,
    RedisSubscriptionOptions, RedisTlsMode, ResolvedSecret, SecretPromptPort,
    SecretResolutionError, SortDirection, SortKey, TypedCondition,
    load_relation_structure as load_structure_snapshot, parse_bind_text,
    resolve_for_connect_with_ports,
};
use tablerock_files::{
    CsvStreamLimits, CsvTable, CsvValueType, csv_to_typed_insert_changes, stream_csv_batches,
    validate_insert_batch_size, write_atomic,
};
use tablerock_persistence::{
    HistoryAppend, HistoryOutcomeClass, HistoryRetention, PersistenceActor, ProfileOrderUpdate,
    SavedQueryUpsert, SqlFileFacts, external_change_detected, read_sql_file, write_sql_file_atomic,
};
use tablerock_tools::{
    PgToolRunOutcome, ToolStatus, cancel_channel, discover_tool, run_pg_dump_configured,
    run_pg_restore_configured, validate_dump_path,
};
use zeroize::Zeroizing;

use crate::{
    error::{BridgeError, catch_entry},
    ids::{
        IdFactory, catalog_node_bytes, catalog_node_from_bytes, operation_bytes,
        operation_from_bytes, result_from_bytes, review_token_bytes, review_token_from_bytes,
        session_bytes, session_from_bytes,
    },
    page_limits::default_page_limits,
    runtime::RuntimeOwner,
};

const CSV_IMPORT_MAX_FILE_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const CSV_IMPORT_MAX_ROWS: u64 = 100_000_000;
const CSV_IMPORT_MAX_CELL_BYTES: usize = 64 * 1024;
const CSV_IMPORT_PREVIEW_ROWS: usize = 100;
const CSV_IMPORT_BATCH_ROWS: usize = 500;
static CSV_IMPORT_SPOOL_NONCE: AtomicU64 = AtomicU64::new(1);

const MAX_EVENT_LOG: usize = 4_096;
const MAX_EVENT_BATCH: u32 = 256;
const MAX_SESSIONS: usize = 64;

/// Connection parameters for the bridge open path (proof + harness).
///
/// Password is never included in Debug output.
#[derive(Clone, uniffi::Record)]
pub struct OpenParams {
    /// `postgresql`, `clickhouse`, or `redis`.
    pub engine: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
    /// `off`, `verify_ca`, or `verify_full`.
    pub tls_mode: String,
}

impl std::fmt::Debug for OpenParams {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OpenParams")
            .field("engine", &self.engine)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("database", &self.database)
            .field("user", &self.user)
            .field("password", &"<redacted>")
            .field("tls_mode", &self.tls_mode)
            .finish()
    }
}

/// Submits one coarse command. Intent-specific fields are optional by kind.
#[derive(Debug, Clone, uniffi::Record)]
pub struct SubmitSpec {
    /// `execute`, `fetch_page`, or `probe`.
    pub intent: String,
    /// Session returned by `open`.
    pub session_id: Vec<u8>,
    /// SQL/Redis command text for execute/probe intents.
    pub statement: Option<String>,
    /// Result id for fetch_page (16 bytes).
    pub result_id: Option<Vec<u8>>,
    /// Start row for fetch_page.
    pub start_row: Option<u64>,
    /// Page row budget for execute/fetch.
    pub row_count: Option<u32>,
    /// Expected aggregate revision (context scope).
    pub expected_revision: u64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeNamedParameterPlan {
    pub names: Vec<String>,
}

#[derive(Clone, uniffi::Record)]
pub struct BridgeQueryParameter {
    pub name: String,
    /// `text`, `integer`, `float`, `boolean`, or `null`.
    pub kind: String,
    /// Absent only for `null`.
    pub value: Option<String>,
}

impl std::fmt::Debug for BridgeQueryParameter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BridgeQueryParameter")
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("value", &self.value.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

/// One native object-browse sort key. Rust validates and quotes the column.
#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeBrowseSort {
    pub column: String,
    /// `asc` or `desc`.
    pub direction: String,
}

/// One native object-browse typed filter. Values remain separate from SQL.
#[derive(Clone, uniffi::Record)]
pub struct BridgeBrowseFilter {
    pub column: String,
    /// Stable operator label (`eq`, `ne`, `lt`, `le`, `gt`, `ge`, `like`,
    /// `ilike`, `not_like`, `not_ilike`, `is_null`, or `is_not_null`).
    pub operator: String,
    pub value: Option<String>,
}

const MAX_BROWSE_IDENTIFIER_BYTES: usize = 1_024;
const MAX_BROWSE_VALUE_BYTES: usize = 64 * 1_024;

impl std::fmt::Debug for BridgeBrowseFilter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BridgeBrowseFilter")
            .field("column", &self.column)
            .field("operator", &self.operator)
            .field("value", &self.value.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

/// One saved filter preset for the catalog object selected by opaque id.
#[derive(Clone, uniffi::Record)]
pub struct BridgeSavedFilterPreset {
    pub name: String,
    pub filters: Vec<BridgeBrowseFilter>,
    pub raw_where: Option<String>,
}

impl std::fmt::Debug for BridgeSavedFilterPreset {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BridgeSavedFilterPreset")
            .field("name", &self.name)
            .field("filter_count", &self.filters.len())
            .field("raw_where", &self.raw_where.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeEventRecord {
    pub sequence: u64,
    pub operation_id: Vec<u8>,
    /// Stable kind label: `started`, `progress`, `page`, `terminal`, `cancel_dispatched`.
    pub kind: String,
    pub outcome: Option<String>,
    pub rows: Option<u64>,
    pub bytes: Option<u64>,
    /// When kind is `page`, the encoded `ResultPage` v1 payload.
    pub page_bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeEventBatch {
    pub next_cursor: u64,
    pub events: Vec<BridgeEventRecord>,
    pub resync_required: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct CancelOutcome {
    pub core: String,
    pub runtime: Option<String>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct ShutdownOutcome {
    pub core: String,
    pub active_operations: u32,
}

/// Safe summary of a handle-based mutation apply (no SQL, no cell values).
#[derive(Debug, Clone, uniffi::Record)]
pub struct ApplyOutcome {
    pub transaction: String,
    pub change_count: u32,
    pub applied_count: u32,
    pub conflict_count: u32,
    pub failed_count: u32,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeConnectionTestReport {
    pub identity: String,
    pub tls_outcome: String,
    pub elapsed_millis: u64,
}

/// Safe live-session health projection. No server text or credentials cross UniFFI.
#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeSessionHealth {
    pub state: String,
    pub server_reachable: bool,
    pub elapsed_millis: Option<u64>,
    pub authentication_stopped: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeReconnectPlan {
    pub action: String,
    pub delay_millis: Option<u64>,
    pub restore_last_context: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeReconnectAttempt {
    pub state: String,
    pub session_id: Option<Vec<u8>>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeHistoryItem {
    pub history_id: i64,
    pub engine: String,
    pub database_name: String,
    pub schema_name: Option<String>,
    pub statement_text: Option<String>,
    pub outcome: String,
    pub created_at: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeSavedQueryItem {
    pub query_id: i64,
    pub name: String,
    pub engine: String,
    pub statement_text: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct BridgeSqlFile {
    pub path: String,
    pub statement_text: String,
    pub modified_nanos: Option<u64>,
    pub len: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct BridgeWorkspaceTab {
    pub title: String,
    pub statement_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct BridgeSessionIntent {
    pub database: String,
    pub schema: Option<String>,
    pub selected_tab: u32,
    pub tabs: Vec<BridgeWorkspaceTab>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct BridgeNativeWindowIntent {
    pub profile_id: Vec<u8>,
    pub intent: BridgeSessionIntent,
}

/// One Rust-owned catalog node. Swift renders these facts and returns only opaque ids.
#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeCatalogNode {
    pub id_bytes: Vec<u8>,
    pub parent_id_bytes: Option<Vec<u8>>,
    pub depth: u16,
    pub name: String,
    pub kind: String,
    pub children_state: String,
    pub expandable: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeCsvRow {
    pub cells: Vec<String>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeCsvImportPreview {
    pub path: String,
    pub headers: Vec<String>,
    pub rows: Vec<BridgeCsvRow>,
    pub total_rows: u32,
    pub formula_like_cells: u32,
    pub fingerprint: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeCsvImportRequest {
    pub session_id: Vec<u8>,
    pub catalog_node_id: Vec<u8>,
    pub path: String,
    pub mapped_columns: Vec<String>,
    pub mapped_types: Vec<String>,
    pub expected_fingerprint: String,
    pub now_ms: u64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeCsvImportReview {
    pub token_id: Vec<u8>,
    pub target: String,
    pub row_count: u32,
    pub column_count: u32,
    pub formula_like_cells: u32,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeCsvImportProgress {
    pub operation_id: Vec<u8>,
    pub phase: String,
    pub completed_rows: u64,
    pub total_rows: u64,
    pub applied_rows: u64,
    pub conflict_rows: u64,
    pub failed_rows: u64,
    pub errors: Vec<String>,
    pub errors_truncated: bool,
    pub summary: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeRelationColumn {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_expression: Option<String>,
    pub comment: Option<String>,
    pub primary_key: bool,
    pub sorting_key: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeRelationFact {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeRelationIndex {
    pub kind: String,
    pub name: String,
    pub definition: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeRelationConstraint {
    pub kind: String,
    pub name: String,
    pub definition: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeRelationStructure {
    pub engine: String,
    pub namespace: String,
    pub relation: String,
    pub columns: Vec<BridgeRelationColumn>,
    pub indexes: Vec<BridgeRelationIndex>,
    pub constraints: Vec<BridgeRelationConstraint>,
    pub facts: Vec<BridgeRelationFact>,
    pub ddl: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeRedisKeyView {
    pub kind: String,
    pub lines: Vec<String>,
    pub next_skip: Option<u64>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeRedisOverview {
    pub sampled_at_ms: u64,
    pub lines: Vec<String>,
}

/// Bounded presentation snapshot for one supervised Redis subscription.
#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeRedisSubscriptionStatus {
    pub operation_id: Vec<u8>,
    pub selector: String,
    pub pattern: bool,
    pub phase: String,
    pub messages: Vec<String>,
    pub total_received: u64,
    pub discontinuities: u64,
    pub summary: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgePostgresActivityRow {
    pub pid: i32,
    pub user: String,
    pub application: String,
    pub state: String,
    pub query_preview: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeRelationshipEdge {
    pub from_schema: String,
    pub from_table: String,
    pub from_column: String,
    pub to_schema: String,
    pub to_table: String,
    pub to_column: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeRelationshipSnapshot {
    pub namespace: String,
    pub relation: String,
    pub edges: Vec<BridgeRelationshipEdge>,
    pub truncated: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeRoleMembership {
    pub role: String,
    pub member: String,
    pub inherit_option: bool,
    pub admin_option: bool,
    pub set_option: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeRolePrivilege {
    pub grantee: String,
    pub privilege: String,
    pub object: String,
    pub grantable: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeRoleSnapshot {
    pub current_user: String,
    pub roles: Vec<String>,
    pub memberships: Vec<BridgeRoleMembership>,
    pub effective_roles: Vec<String>,
    pub cycle_edges: Vec<String>,
    pub privileges: Vec<BridgeRolePrivilege>,
    pub privilege_scope: Option<String>,
    pub privileges_unavailable: bool,
    pub truncated: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeRoleChangeRequest {
    pub session_id: Vec<u8>,
    pub catalog_node_id: Option<Vec<u8>>,
    pub kind: String,
    pub role: String,
    pub member_or_grantee: String,
    pub privilege: String,
    pub now_ms: u64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeRoleChangeReview {
    pub token_id: Vec<u8>,
    pub summary: String,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeDdlChangeRequest {
    pub session_id: Vec<u8>,
    pub catalog_node_id: Vec<u8>,
    pub kind: String,
    pub object_name: String,
    pub definition: String,
    pub now_ms: u64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeDdlChangeReview {
    pub token_id: Vec<u8>,
    pub preview: String,
    pub destructive: bool,
    pub rollback_summary: String,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeTableOperationRequest {
    pub session_id: Vec<u8>,
    pub catalog_node_id: Vec<u8>,
    /// `rename`, `truncate`, `drop`, `vacuum`, `analyze`, or `optimize`.
    pub kind: String,
    pub new_name: String,
    pub now_ms: u64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeTableOperationReview {
    pub token_id: Vec<u8>,
    pub target: String,
    pub preview: String,
    pub destructive: bool,
    pub confirmation: String,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeBackendSignalOutcome {
    pub kind: String,
    pub pid: i32,
    pub acknowledged: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgePostgresToolProbe {
    pub kind: String,
    pub available: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgePostgresToolStatus {
    pub operation_id: Vec<u8>,
    pub kind: String,
    pub phase: String,
    pub summary: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgePostgresToolRequest {
    pub session_id: Vec<u8>,
    pub kind: String,
    pub tool_path: String,
    pub file_path: String,
    pub content: String,
    pub clean: bool,
    pub no_owner: bool,
}

#[derive(Clone)]
struct PostgresToolConnection {
    host: String,
    port: u16,
    database: String,
    user: String,
    password: Arc<Zeroizing<String>>,
}

struct PostgresToolTask {
    session_id: SessionId,
    kind: String,
    phase: String,
    summary: String,
    cancel: tokio::sync::watch::Sender<bool>,
}

struct RedisSubscriptionTask {
    session_id: SessionId,
    selector: String,
    pattern: bool,
    phase: String,
    messages: VecDeque<String>,
    total_received: u64,
    discontinuities: u64,
    summary: String,
    cancel: tokio::sync::watch::Sender<bool>,
}

struct CsvImportTask {
    session_id: SessionId,
    control: MutationApplyControl,
    phase: String,
    completed_rows: u64,
    total_rows: u64,
    applied_rows: u64,
    conflict_rows: u64,
    failed_rows: u64,
    errors: Vec<String>,
    errors_truncated: bool,
    summary: String,
}

struct CsvImportReviewEntry {
    session_id: SessionId,
    scope: OperationScope,
    revision: Revision,
    target: MutationTarget,
    frozen_path: PathBuf,
    mapped_columns: Vec<String>,
    value_types: Vec<CsvValueType>,
    row_count: u64,
    expires_at_ms: u64,
}

impl Drop for CsvImportReviewEntry {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.frozen_path);
    }
}

struct RoleReviewEntry {
    session_id: SessionId,
    reviewed: ReviewedRoleChangePlan,
    expires_at_ms: u64,
}

struct DdlReviewEntry {
    session_id: SessionId,
    plan: DdlPlan,
    expires_at_ms: u64,
    confirmation: Option<String>,
}

#[derive(Clone)]
struct RegisteredSession {
    profile_id: tablerock_core::ProfileId,
    session_id: SessionId,
    context_id: tablerock_core::ContextId,
    engine: Engine,
    database: BoundedText,
    postgres_tool_connection: Option<PostgresToolConnection>,
    /// Expected context revision tracked by the bridge.
    context_revision: Revision,
}

struct BridgeInner {
    service: EngineService,
    results: ResultStore,
    reviews: MutationReviewRegistry,
    csv_import_reviews: BTreeMap<ReviewTokenId, CsvImportReviewEntry>,
    role_reviews: BTreeMap<ReviewTokenId, RoleReviewEntry>,
    ddl_reviews: BTreeMap<ReviewTokenId, DdlReviewEntry>,
    sessions: BTreeMap<SessionId, RegisteredSession>,
    /// Operation -> result identity used when admitting streamed pages.
    operation_results: BTreeMap<OperationId, PageIdentity>,
    operation_history: BTreeMap<OperationId, HistoryAppend>,
    operation_copy_identity: BTreeMap<OperationId, CopyIdentity>,
    history_retention: HistoryRetention,
    ids: IdFactory,
    events: VecDeque<BridgeEventRecord>,
    /// Absolute sequence of the next event to append (also next_cursor when caught up).
    next_sequence: u64,
    /// Lowest sequence still retained in `events`.
    first_sequence: u64,
    accepting: bool,
    /// Optional local-only profile store (never logs secrets).
    persistence: Option<PersistenceActor>,
    catalog_nodes: BTreeMap<(SessionId, CatalogNodeId), CatalogNode>,
    copy_identities: BTreeMap<tablerock_core::ResultId, CopyIdentity>,
    support_bundle: SupportBundle,
}

impl Drop for BridgeInner {
    fn drop(&mut self) {
        for review in self.csv_import_reviews.values() {
            let _ = std::fs::remove_file(&review.frozen_path);
        }
    }
}

#[derive(Clone)]
struct CopyIdentity {
    schema: String,
    table: String,
    identity_columns: Vec<String>,
    insertable: bool,
}

/// One saved profile row for the native connection screen.
#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeProfileItem {
    /// 16-byte ProfileId (same form `open_profile` accepts).
    pub id_bytes: Vec<u8>,
    pub revision: u64,
    pub name: String,
    pub engine: String,
    pub group: Option<String>,
    pub favorite: bool,
    pub saved_order: u32,
    pub host: Option<String>,
    pub port: Option<String>,
    pub context: Option<String>,
    pub safety_mode: String,
    pub environment: Option<String>,
    pub production_warning: bool,
    pub dangerous_plaintext: bool,
    /// At least one live bridge session still owns this saved profile id.
    pub connected: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeProfileGroup {
    pub name: String,
    pub alphabetical: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeProfileOrderItem {
    pub id_bytes: Vec<u8>,
    pub expected_revision: u64,
}

/// Editable saved-profile projection. Secret references are IDs only; resolved
/// values never cross the bridge. Existing plaintext is represented by
/// `has_stored_password`, not returned.
#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeProfileDraft {
    pub id_bytes: Option<Vec<u8>>,
    pub revision: u64,
    pub engine: String,
    pub name: String,
    pub group: String,
    pub environment: String,
    pub host: String,
    pub port: String,
    pub database: String,
    pub username: String,
    pub password_source: String,
    pub password_value: String,
    pub password_reference: Option<Vec<u8>>,
    pub has_stored_password: bool,
    pub plaintext_acknowledged: bool,
    pub tls_mode: String,
    pub safety_mode: String,
}

/// Process-scoped UniFFI facade. One instance owns the multi-thread runtime.
#[derive(uniffi::Object)]
pub struct TableRockBridge {
    runtime: RuntimeOwner,
    inner: Mutex<Option<BridgeInner>>,
    postgres_tools: Arc<Mutex<BTreeMap<OperationId, PostgresToolTask>>>,
    redis_subscriptions: Arc<Mutex<BTreeMap<OperationId, RedisSubscriptionTask>>>,
    csv_imports: Arc<Mutex<BTreeMap<OperationId, CsvImportTask>>>,
}

#[uniffi::export]
impl TableRockBridge {
    #[uniffi::constructor]
    #[must_use]
    pub fn create() -> Arc<Self> {
        Arc::new(Self {
            runtime: RuntimeOwner::new(),
            inner: Mutex::new(None),
            postgres_tools: Arc::new(Mutex::new(BTreeMap::new())),
            redis_subscriptions: Arc::new(Mutex::new(BTreeMap::new())),
            csv_imports: Arc::new(Mutex::new(BTreeMap::new())),
        })
    }

    /// Ensures the Tokio runtime and service coordinator exist (idempotent).
    pub fn ensure_runtime(&self) -> Result<(), BridgeError> {
        catch_entry(|| self.ensure_runtime_inner())
    }

    /// Opens a session from connection parameters and returns a 16-byte session id.
    pub fn open(&self, params: OpenParams) -> Result<Vec<u8>, BridgeError> {
        catch_entry(|| self.open_inner(params))
    }

    /// Attach a local-only persistence file for profile-backed open (idempotent replace).
    pub fn configure_persistence(&self, path: String) -> Result<(), BridgeError> {
        catch_entry(|| self.configure_persistence_inner(path))
    }

    /// Open by saved profile id (16 bytes). Literal host/port required; password may be
    /// supplied as an override when the stored source is not inline-resolvable.
    pub fn open_profile(
        &self,
        profile_id: Vec<u8>,
        password_override: Option<String>,
    ) -> Result<Vec<u8>, BridgeError> {
        catch_entry(|| {
            self.open_profile_inner(profile_id, password_override.map(String::into_bytes))
        })
    }

    /// Native credential path: transient bytes cross FFI without observable text state.
    pub fn open_profile_with_secret(
        &self,
        profile_id: Vec<u8>,
        secret_override: Option<Vec<u8>>,
    ) -> Result<Vec<u8>, BridgeError> {
        catch_entry(|| self.open_profile_inner(profile_id, secret_override))
    }

    /// Lists saved profiles (all engines) for the native connection screen.
    /// Requires `configure_persistence` first.
    pub fn list_profiles(&self) -> Result<Vec<BridgeProfileItem>, BridgeError> {
        catch_entry(|| self.list_profiles_inner(None))
    }

    /// Rust-owned normalized profile search across name, endpoint, database, and group.
    pub fn search_profiles(
        &self,
        search: Option<String>,
    ) -> Result<Vec<BridgeProfileItem>, BridgeError> {
        catch_entry(|| self.list_profiles_inner(search))
    }

    /// Loads one editable profile without resolving or returning credentials.
    pub fn get_profile_draft(
        &self,
        profile_id: Vec<u8>,
    ) -> Result<BridgeProfileDraft, BridgeError> {
        catch_entry(|| self.get_profile_draft_inner(profile_id))
    }

    /// Parses a database URL through the shared Rust safety policy and returns
    /// an unsaved, reviewable profile draft. A URL password is never persisted
    /// by this operation; native presentation must choose its destination.
    pub fn parse_connection_url_draft(
        &self,
        input: String,
    ) -> Result<BridgeProfileDraft, BridgeError> {
        catch_entry(|| {
            let parsed = parse_connection_url(&input)
                .map_err(|error| BridgeError::rejected("connection-url", error.to_string()))?;
            let has_password = parsed.password.is_some();
            Ok(BridgeProfileDraft {
                id_bytes: None,
                revision: 0,
                engine: engine_label(parsed.engine).to_owned(),
                name: String::new(),
                group: String::new(),
                environment: String::new(),
                host: parsed.host,
                port: parsed.port.to_string(),
                database: parsed.database,
                username: parsed.username,
                password_source: if has_password { "keychain" } else { "prompt" }.to_owned(),
                password_value: parsed.password.unwrap_or_default(),
                password_reference: None,
                has_stored_password: false,
                plaintext_acknowledged: false,
                tls_mode: match parsed.tls {
                    tablerock_core::ConnectionUrlTls::Off => "off",
                    tablerock_core::ConnectionUrlTls::Required => "verify_full",
                }
                .to_owned(),
                safety_mode: "confirm_writes".to_owned(),
            })
        })
    }

    /// Creates or revision-checked replaces one saved profile.
    pub fn save_profile(&self, draft: BridgeProfileDraft) -> Result<Vec<u8>, BridgeError> {
        catch_entry(|| self.save_profile_inner(draft))
    }

    /// Revision-checked removal; active sessions remain alive.
    pub fn delete_profile(
        &self,
        profile_id: Vec<u8>,
        expected_revision: u64,
    ) -> Result<(), BridgeError> {
        catch_entry(|| self.delete_profile_inner(profile_id, expected_revision))
    }

    /// Connects, describes, and disconnects without changing persistence.
    pub fn test_profile(
        &self,
        profile_id: Vec<u8>,
        password_override: Option<String>,
    ) -> Result<BridgeConnectionTestReport, BridgeError> {
        catch_entry(|| {
            self.test_profile_inner(profile_id, password_override.map(String::into_bytes))
        })
    }

    pub fn test_profile_with_secret(
        &self,
        profile_id: Vec<u8>,
        secret_override: Option<Vec<u8>>,
    ) -> Result<BridgeConnectionTestReport, BridgeError> {
        catch_entry(|| self.test_profile_inner(profile_id, secret_override))
    }

    pub fn list_profile_groups(&self) -> Result<Vec<BridgeProfileGroup>, BridgeError> {
        catch_entry(|| self.list_profile_groups_inner())
    }

    /// Lists newest local query-history entries with optional SQL-text search.
    pub fn list_history(
        &self,
        search: Option<String>,
        limit: u32,
    ) -> Result<Vec<BridgeHistoryItem>, BridgeError> {
        catch_entry(|| self.list_history_inner(search, limit))
    }

    /// Sets process history retention for subsequent operations.
    pub fn set_history_retention(&self, retention: String) -> Result<(), BridgeError> {
        catch_entry(|| self.set_history_retention_inner(retention))
    }

    pub fn history_retention(&self) -> Result<String, BridgeError> {
        catch_entry(|| self.history_retention_inner())
    }

    pub fn list_saved_queries(
        &self,
        engine: Option<String>,
        search: Option<String>,
    ) -> Result<Vec<BridgeSavedQueryItem>, BridgeError> {
        catch_entry(|| self.list_saved_queries_inner(engine, search))
    }

    pub fn save_query(
        &self,
        name: String,
        engine: String,
        statement_text: String,
    ) -> Result<i64, BridgeError> {
        catch_entry(|| self.save_query_inner(name, engine, statement_text))
    }

    pub fn delete_saved_query(&self, query_id: i64) -> Result<bool, BridgeError> {
        catch_entry(|| self.delete_saved_query_inner(query_id))
    }

    pub fn list_catalog_filter_presets(
        &self,
        session_id: Vec<u8>,
        catalog_node_id: Vec<u8>,
    ) -> Result<Vec<BridgeSavedFilterPreset>, BridgeError> {
        catch_entry(|| self.list_catalog_filter_presets_inner(session_id, catalog_node_id))
    }

    pub fn save_catalog_filter_preset(
        &self,
        session_id: Vec<u8>,
        catalog_node_id: Vec<u8>,
        preset: BridgeSavedFilterPreset,
    ) -> Result<(), BridgeError> {
        catch_entry(|| self.save_catalog_filter_preset_inner(session_id, catalog_node_id, preset))
    }

    pub fn read_sql_file(&self, path: String) -> Result<BridgeSqlFile, BridgeError> {
        catch_entry(|| read_bridge_sql_file(&path))
    }

    pub fn write_sql_file(
        &self,
        path: String,
        statement_text: String,
        expected_modified_nanos: Option<u64>,
        expected_len: Option<u64>,
        overwrite_external_change: bool,
    ) -> Result<BridgeSqlFile, BridgeError> {
        catch_entry(|| {
            write_bridge_sql_file(
                &path,
                &statement_text,
                expected_modified_nanos,
                expected_len,
                overwrite_external_change,
            )
        })
    }

    pub fn put_session_intent(
        &self,
        profile_id: Vec<u8>,
        intent: BridgeSessionIntent,
    ) -> Result<(), BridgeError> {
        catch_entry(|| self.put_session_intent_inner(profile_id, intent))
    }

    pub fn get_session_intent(
        &self,
        profile_id: Vec<u8>,
    ) -> Result<Option<BridgeSessionIntent>, BridgeError> {
        catch_entry(|| self.get_session_intent_inner(profile_id))
    }

    pub fn delete_session_intent(&self, profile_id: Vec<u8>) -> Result<(), BridgeError> {
        catch_entry(|| self.delete_session_intent_inner(profile_id))
    }

    pub fn put_native_window_intent(
        &self,
        window_id: String,
        profile_id: Vec<u8>,
        intent: BridgeSessionIntent,
    ) -> Result<(), BridgeError> {
        catch_entry(|| self.put_native_window_intent_inner(window_id, profile_id, intent))
    }

    pub fn get_native_window_intent(
        &self,
        window_id: String,
    ) -> Result<Option<BridgeNativeWindowIntent>, BridgeError> {
        catch_entry(|| self.get_native_window_intent_inner(window_id))
    }

    pub fn delete_native_window_intent(&self, window_id: String) -> Result<(), BridgeError> {
        catch_entry(|| self.delete_native_window_intent_inner(window_id))
    }

    pub fn create_profile_group(&self, name: String) -> Result<(), BridgeError> {
        catch_entry(|| self.create_profile_group_inner(name))
    }

    pub fn rename_profile_group(
        &self,
        old_name: String,
        new_name: String,
    ) -> Result<u32, BridgeError> {
        catch_entry(|| self.rename_profile_group_inner(old_name, new_name))
    }

    pub fn delete_profile_group(&self, name: String) -> Result<u32, BridgeError> {
        catch_entry(|| self.delete_profile_group_inner(name))
    }

    pub fn set_profile_group_alphabetical(
        &self,
        name: String,
        alphabetical: bool,
    ) -> Result<(), BridgeError> {
        catch_entry(|| self.set_profile_group_alphabetical_inner(name, alphabetical))
    }

    pub fn set_profile_favorite(
        &self,
        profile_id: Vec<u8>,
        expected_revision: u64,
        favorite: bool,
    ) -> Result<(), BridgeError> {
        catch_entry(|| self.set_profile_favorite_inner(profile_id, expected_revision, favorite))
    }

    pub fn reorder_profiles(
        &self,
        group: Option<String>,
        ordered: Vec<BridgeProfileOrderItem>,
    ) -> Result<(), BridgeError> {
        catch_entry(|| self.reorder_profiles_inner(group, ordered))
    }

    /// Load one typed catalog level. `parent_node_id` is an opaque id previously
    /// returned by this method; Swift never chooses engine requests or names.
    pub fn refresh_catalog(
        &self,
        session_id: Vec<u8>,
        parent_node_id: Option<Vec<u8>>,
    ) -> Result<Vec<BridgeCatalogNode>, BridgeError> {
        catch_entry(|| self.refresh_catalog_inner(session_id, parent_node_id))
    }

    pub fn submit_catalog_browse(
        &self,
        session_id: Vec<u8>,
        catalog_node_id: Vec<u8>,
        row_count: u32,
    ) -> Result<Vec<u8>, BridgeError> {
        catch_entry(|| {
            self.submit_catalog_browse_inner(
                session_id,
                catalog_node_id,
                Vec::new(),
                Vec::new(),
                None,
                row_count,
            )
        })
    }

    pub fn submit_catalog_browse_with_sort(
        &self,
        session_id: Vec<u8>,
        catalog_node_id: Vec<u8>,
        sort: Vec<BridgeBrowseSort>,
        row_count: u32,
    ) -> Result<Vec<u8>, BridgeError> {
        catch_entry(|| {
            self.submit_catalog_browse_inner(
                session_id,
                catalog_node_id,
                sort,
                Vec::new(),
                None,
                row_count,
            )
        })
    }

    pub fn submit_catalog_browse_with_plan(
        &self,
        session_id: Vec<u8>,
        catalog_node_id: Vec<u8>,
        sort: Vec<BridgeBrowseSort>,
        filters: Vec<BridgeBrowseFilter>,
        raw_where: Option<String>,
        row_count: u32,
    ) -> Result<Vec<u8>, BridgeError> {
        catch_entry(|| {
            self.submit_catalog_browse_inner(
                session_id,
                catalog_node_id,
                sort,
                filters,
                raw_where,
                row_count,
            )
        })
    }

    /// Loads a bounded typed structure snapshot for one cached catalog object.
    pub fn relation_structure(
        &self,
        session_id: Vec<u8>,
        catalog_node_id: Vec<u8>,
    ) -> Result<BridgeRelationStructure, BridgeError> {
        catch_entry(|| self.relation_structure_inner(session_id, catalog_node_id))
    }

    /// Loads one bounded type-specific Redis key view from an opaque catalog node.
    pub fn redis_key_view(
        &self,
        session_id: Vec<u8>,
        catalog_node_id: Vec<u8>,
        collection_skip: u64,
    ) -> Result<BridgeRedisKeyView, BridgeError> {
        catch_entry(|| self.redis_key_view_inner(session_id, catalog_node_id, collection_skip))
    }

    /// Loads one bounded, sample-timed Redis INFO overview.
    pub fn redis_overview(&self, session_id: Vec<u8>) -> Result<BridgeRedisOverview, BridgeError> {
        catch_entry(|| self.redis_overview_inner(session_id))
    }

    /// Starts one bounded supervised SUBSCRIBE or PSUBSCRIBE stream.
    pub fn start_redis_subscription(
        &self,
        session_id: Vec<u8>,
        selector: String,
        pattern: bool,
    ) -> Result<Vec<u8>, BridgeError> {
        catch_entry(|| self.start_redis_subscription_inner(session_id, selector, pattern))
    }

    /// Returns the latest bounded message window and delivery-gap count.
    pub fn redis_subscription_status(
        &self,
        operation_id: Vec<u8>,
    ) -> Result<BridgeRedisSubscriptionStatus, BridgeError> {
        catch_entry(|| self.redis_subscription_status_inner(operation_id))
    }

    /// Requests cancellation. Repeated requests are safe.
    pub fn cancel_redis_subscription(&self, operation_id: Vec<u8>) -> Result<bool, BridgeError> {
        catch_entry(|| self.cancel_redis_subscription_inner(operation_id))
    }

    /// Loads a bounded Rust-owned PostgreSQL activity snapshot.
    pub fn postgres_activity(
        &self,
        session_id: Vec<u8>,
    ) -> Result<Vec<BridgePostgresActivityRow>, BridgeError> {
        catch_entry(|| self.postgres_activity_inner(session_id))
    }

    /// Loads a bounded inbound/outbound PostgreSQL FK graph for one catalog object.
    pub fn postgres_relationships(
        &self,
        session_id: Vec<u8>,
        catalog_node_id: Vec<u8>,
    ) -> Result<BridgeRelationshipSnapshot, BridgeError> {
        catch_entry(|| self.postgres_relationships_inner(session_id, catalog_node_id))
    }

    /// Loads a bounded PostgreSQL role snapshot, optionally scoped to one relation.
    pub fn postgres_roles(
        &self,
        session_id: Vec<u8>,
        catalog_node_id: Option<Vec<u8>>,
    ) -> Result<BridgeRoleSnapshot, BridgeError> {
        catch_entry(|| self.postgres_roles_inner(session_id, catalog_node_id))
    }

    /// Freezes one typed role change behind a 60-second consume-once token.
    pub fn stage_postgres_role_change(
        &self,
        request: BridgeRoleChangeRequest,
    ) -> Result<BridgeRoleChangeReview, BridgeError> {
        catch_entry(|| self.stage_postgres_role_change_inner(request))
    }

    /// Consumes and applies one reviewed role change. Failed apply is not retryable.
    pub fn apply_postgres_role_change(
        &self,
        token_id: Vec<u8>,
        session_id: Vec<u8>,
        now_ms: u64,
        confirmed: bool,
    ) -> Result<String, BridgeError> {
        catch_entry(|| {
            self.apply_postgres_role_change_inner(token_id, session_id, now_ms, confirmed)
        })
    }

    /// Discards an unused role-change review token.
    pub fn revoke_postgres_role_change(&self, token_id: Vec<u8>) -> Result<bool, BridgeError> {
        catch_entry(|| self.revoke_postgres_role_change_inner(token_id))
    }

    /// Freezes one typed PostgreSQL structure change behind a 60-second token.
    pub fn stage_ddl_change(
        &self,
        request: BridgeDdlChangeRequest,
    ) -> Result<BridgeDdlChangeReview, BridgeError> {
        catch_entry(|| self.stage_ddl_change_inner(request))
    }

    /// Consumes and applies one reviewed structure change. Failed apply is not retryable.
    pub fn apply_ddl_change(
        &self,
        token_id: Vec<u8>,
        session_id: Vec<u8>,
        now_ms: u64,
        confirmed: bool,
    ) -> Result<String, BridgeError> {
        catch_entry(|| self.apply_ddl_change_inner(token_id, session_id, now_ms, confirmed))
    }

    /// Discards an unused structure-change review token.
    pub fn revoke_ddl_change(&self, token_id: Vec<u8>) -> Result<bool, BridgeError> {
        catch_entry(|| self.revoke_ddl_change_inner(token_id))
    }

    /// Freezes one typed table operation behind target-specific confirmation.
    pub fn stage_table_operation(
        &self,
        request: BridgeTableOperationRequest,
    ) -> Result<BridgeTableOperationReview, BridgeError> {
        catch_entry(|| self.stage_table_operation_inner(request))
    }

    /// Consumes one reviewed table operation before database I/O.
    pub fn apply_table_operation(
        &self,
        token_id: Vec<u8>,
        session_id: Vec<u8>,
        now_ms: u64,
        confirmation: String,
    ) -> Result<String, BridgeError> {
        catch_entry(|| self.apply_table_operation_inner(token_id, session_id, now_ms, confirmation))
    }

    pub fn revoke_table_operation(&self, token_id: Vec<u8>) -> Result<bool, BridgeError> {
        catch_entry(|| self.revoke_ddl_change_inner(token_id))
    }

    /// Signals one PostgreSQL backend. Kind is exactly `cancel` or `terminate`.
    pub fn signal_postgres_backend(
        &self,
        session_id: Vec<u8>,
        kind: String,
        pid: i32,
    ) -> Result<BridgeBackendSignalOutcome, BridgeError> {
        catch_entry(|| self.signal_postgres_backend_inner(session_id, kind, pid))
    }

    /// Probes an exact PostgreSQL client tool without invoking a shell.
    pub fn probe_postgres_tool(
        &self,
        kind: String,
        explicit_path: Option<String>,
    ) -> Result<BridgePostgresToolProbe, BridgeError> {
        catch_entry(|| probe_postgres_tool_inner(kind, explicit_path))
    }

    /// Starts a supervised dump or restore against the connected endpoint.
    pub fn start_postgres_tool(
        &self,
        request: BridgePostgresToolRequest,
    ) -> Result<Vec<u8>, BridgeError> {
        catch_entry(|| self.start_postgres_tool_inner(request))
    }

    /// Returns one bounded process status projection.
    pub fn postgres_tool_status(
        &self,
        operation_id: Vec<u8>,
    ) -> Result<BridgePostgresToolStatus, BridgeError> {
        catch_entry(|| self.postgres_tool_status_inner(operation_id))
    }

    /// Requests cancellation; repeated requests remain safe.
    pub fn cancel_postgres_tool(&self, operation_id: Vec<u8>) -> Result<bool, BridgeError> {
        catch_entry(|| self.cancel_postgres_tool_inner(operation_id))
    }

    /// Formats resident Rust-owned result pages for clipboard/export.
    /// Scope is `cell`, `row`, or `loaded`; format is csv/tsv/json/markdown/sql_insert/sql_update.
    pub fn format_result_copy(
        &self,
        result_id: Vec<u8>,
        revision: u64,
        scope: String,
        row: Option<u64>,
        column: Option<u32>,
        format: String,
    ) -> Result<String, BridgeError> {
        catch_entry(|| {
            self.format_result_copy_inner(result_id, revision, scope, row, column, format)
        })
    }

    /// Atomically exports all resident rows through the shared typed formatter.
    pub fn export_loaded_result(
        &self,
        result_id: Vec<u8>,
        revision: u64,
        format: String,
        path: String,
    ) -> Result<u64, BridgeError> {
        catch_entry(|| {
            if !Path::new(&path).is_absolute() {
                return Err(BridgeError::rejected(
                    "export-path",
                    "native export path must be absolute",
                ));
            }
            let payload = self.format_result_copy_inner(
                result_id,
                revision,
                "loaded".into(),
                None,
                None,
                format,
            )?;
            write_atomic(Path::new(&path), payload.as_bytes()).map_err(|error| {
                BridgeError::rejected("export-file", format!("atomic export failed: {error}"))
            })
        })
    }

    /// Atomically exports the closed safe-schema support manifest.
    pub fn export_support_bundle(&self, path: String) -> Result<u64, BridgeError> {
        catch_entry(|| {
            if !Path::new(&path).is_absolute() {
                return Err(BridgeError::rejected(
                    "support-path",
                    "native support path must be absolute",
                ));
            }
            self.ensure_runtime_inner()?;
            let guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_ref().ok_or(BridgeError::RuntimeUnavailable)?;
            let payload = inner.support_bundle.render(env!("CARGO_PKG_VERSION"));
            write_atomic(Path::new(&path), payload.as_bytes()).map_err(|error| {
                BridgeError::rejected(
                    "support-file",
                    format!("atomic support export failed: {error}"),
                )
            })
        })
    }

    /// Reads a bounded UTF-8 CSV file for native mapping and review.
    pub fn preview_csv_import(&self, path: String) -> Result<BridgeCsvImportPreview, BridgeError> {
        catch_entry(|| preview_csv_import_inner(path))
    }

    /// Freezes a mapped CSV insert plan behind a single-use review token.
    pub fn stage_csv_import(
        &self,
        request: BridgeCsvImportRequest,
    ) -> Result<BridgeCsvImportReview, BridgeError> {
        catch_entry(|| self.stage_csv_import_inner(request))
    }

    /// Stage a probe mutation + register a single-use review token for the
    /// native edit-safety demo. Returns the token id for `authorize_review_token`
    /// / `apply_review_token`. Wraps the conformance staging seam with sensible
    /// defaults (60 s expiry, `public.users`, locator 1).
    pub fn stage_probe_review(
        &self,
        session_id: Vec<u8>,
        now_ms: u64,
    ) -> Result<Vec<u8>, BridgeError> {
        catch_entry(|| {
            self.insert_reviewed_probe_inner(
                session_id,
                now_ms,
                now_ms + 60_000,
                now_ms,
                "users".into(),
                1,
            )
        })
    }

    /// Submits a command and returns a 16-byte operation id.
    pub fn submit(&self, spec: SubmitSpec) -> Result<Vec<u8>, BridgeError> {
        catch_entry(|| self.submit_inner(spec))
    }

    /// Inspects named placeholders without retaining statement text.
    pub fn inspect_named_parameters(
        &self,
        statement: String,
    ) -> Result<BridgeNamedParameterPlan, BridgeError> {
        catch_entry(|| {
            let plan = rewrite_named_params(&statement)
                .map_err(|error| BridgeError::rejected("named-parameters", error.to_string()))?;
            Ok(BridgeNamedParameterPlan { names: plan.names })
        })
    }

    /// Rewrites named placeholders and submits separately typed values.
    pub fn submit_named(
        &self,
        spec: SubmitSpec,
        bindings: Vec<BridgeQueryParameter>,
    ) -> Result<Vec<u8>, BridgeError> {
        catch_entry(|| self.submit_named_inner(spec, bindings))
    }

    /// Pumps driver updates for `operation_id` until a terminal fact or no pending work.
    pub fn pump(&self, operation_id: Vec<u8>) -> Result<(), BridgeError> {
        catch_entry(|| self.pump_inner(operation_id))
    }

    /// Returns a bounded event batch starting at `cursor` (exclusive of prior delivery).
    pub fn next_events(&self, cursor: u64, maximum: u32) -> Result<BridgeEventBatch, BridgeError> {
        catch_entry(|| self.next_events_inner(cursor, maximum))
    }

    /// Fetches a resident page as version-1 encoded bytes.
    pub fn fetch_page(
        &self,
        result_id: Vec<u8>,
        start_row: u64,
        revision: u64,
    ) -> Result<Vec<u8>, BridgeError> {
        catch_entry(|| self.fetch_page_inner(result_id, start_row, revision))
    }

    pub fn cancel(&self, operation_id: Vec<u8>) -> Result<CancelOutcome, BridgeError> {
        catch_entry(|| self.cancel_inner(operation_id))
    }

    /// Graceful or cancel-active shutdown with a hard drain deadline.
    pub fn shutdown(
        &self,
        cancel_active: bool,
        deadline_ms: u64,
    ) -> Result<ShutdownOutcome, BridgeError> {
        catch_entry(|| self.shutdown_inner(cancel_active, deadline_ms))
    }

    /// Drops the Tokio runtime after service shutdown. Idempotent.
    pub fn destroy_runtime(&self) -> Result<(), BridgeError> {
        catch_entry(|| {
            self.runtime.shutdown()?;
            Ok(())
        })
    }

    /// Test-only: panics inside catch_unwind so callers observe ContainedPanic.
    pub fn panic_probe(&self) -> Result<(), BridgeError> {
        catch_entry(|| {
            panic!("tablerock-ffi panic probe");
        })
    }

    /// Consume-once authorize by review-token handle (never plan bytes).
    ///
    /// Returns the token id bytes on success for correlation; authority is
    /// removed even when later apply fails (core registry contract).
    pub fn authorize_review_token(
        &self,
        token_id: Vec<u8>,
        now_ms: u64,
        session_id: Vec<u8>,
        expected_revision: u64,
    ) -> Result<Vec<u8>, BridgeError> {
        catch_entry(|| {
            self.authorize_review_token_inner(token_id, now_ms, session_id, expected_revision)
        })
    }

    /// Drop a review token without authorizing (operator discard).
    pub fn revoke_review_token(&self, token_id: Vec<u8>) -> Result<bool, BridgeError> {
        catch_entry(|| self.revoke_review_token_inner(token_id))
    }

    /// Consume-once authorize + apply by review-token handle (never plan bytes).
    ///
    /// Token is removed before apply; a failed apply cannot be retried with the
    /// same handle (ambiguous-write non-retry / single-use authority).
    pub fn apply_review_token(
        &self,
        token_id: Vec<u8>,
        now_ms: u64,
        session_id: Vec<u8>,
        expected_revision: u64,
    ) -> Result<ApplyOutcome, BridgeError> {
        catch_entry(|| {
            self.apply_review_token_inner(token_id, now_ms, session_id, expected_revision)
        })
    }

    /// Consume a CSV review and start Rust-owned asynchronous application.
    pub fn start_csv_import_apply(
        &self,
        token_id: Vec<u8>,
        now_ms: u64,
        session_id: Vec<u8>,
    ) -> Result<Vec<u8>, BridgeError> {
        catch_entry(|| self.start_csv_import_apply_inner(token_id, now_ms, session_id))
    }

    /// Poll one bounded CSV import progress/outcome snapshot.
    pub fn csv_import_progress(
        &self,
        operation_id: Vec<u8>,
    ) -> Result<BridgeCsvImportProgress, BridgeError> {
        catch_entry(|| self.csv_import_progress_inner(operation_id))
    }

    /// Request cancellation. The engine observes it at the next safe row boundary.
    pub fn cancel_csv_import(&self, operation_id: Vec<u8>) -> Result<bool, BridgeError> {
        catch_entry(|| self.cancel_csv_import_inner(operation_id))
    }

    /// Remove one terminal import snapshot after the client closes it.
    pub fn dismiss_csv_import(&self, operation_id: Vec<u8>) -> Result<bool, BridgeError> {
        catch_entry(|| self.dismiss_csv_import_inner(operation_id))
    }

    /// Disconnect a session once no operation still holds it.
    pub fn disconnect(&self, session_id: Vec<u8>) -> Result<(), BridgeError> {
        catch_entry(|| self.disconnect_inner(session_id))
    }

    /// Executes one explicit driver health probe for a live session.
    pub fn check_session_health(
        &self,
        session_id: Vec<u8>,
    ) -> Result<BridgeSessionHealth, BridgeError> {
        catch_entry(|| self.check_session_health_inner(session_id))
    }

    /// Returns the saved profile's shared reconnect decision for one attempt.
    pub fn plan_session_reconnect(
        &self,
        session_id: Vec<u8>,
        attempt: u32,
        authentication_stopped: bool,
    ) -> Result<BridgeReconnectPlan, BridgeError> {
        catch_entry(|| {
            self.plan_session_reconnect_inner(session_id, attempt, authentication_stopped)
        })
    }

    /// Opens replacement first, then retires the old saved-profile session.
    pub fn reconnect_saved_session(
        &self,
        session_id: Vec<u8>,
        password_override: Option<String>,
    ) -> Result<BridgeReconnectAttempt, BridgeError> {
        catch_entry(|| {
            self.reconnect_saved_session_inner(
                session_id,
                password_override.map(String::into_bytes),
            )
        })
    }

    pub fn reconnect_saved_session_with_secret(
        &self,
        session_id: Vec<u8>,
        secret_override: Option<Vec<u8>>,
    ) -> Result<BridgeReconnectAttempt, BridgeError> {
        catch_entry(|| self.reconnect_saved_session_inner(session_id, secret_override))
    }
}

impl TableRockBridge {
    fn plan_session_reconnect_inner(
        &self,
        session_id_bytes: Vec<u8>,
        attempt: u32,
        authentication_stopped: bool,
    ) -> Result<BridgeReconnectPlan, BridgeError> {
        let session_id = session_from_bytes(&session_id_bytes)
            .map_err(|_| BridgeError::rejected("session-id", "invalid session id"))?;
        let (preference, restore_last_context) = {
            let guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_ref().ok_or(BridgeError::RuntimeUnavailable)?;
            let profile_id = inner
                .sessions
                .get(&session_id)
                .ok_or(BridgeError::UnknownSession)?
                .profile_id;
            let actor = inner.persistence.as_ref().ok_or_else(|| {
                BridgeError::rejected("reconnect-profile", "session has no saved profile")
            })?;
            let profile = actor
                .get_profile(profile_id)
                .map_err(|error| BridgeError::rejected("reconnect-profile", error.to_string()))?
                .ok_or_else(|| {
                    BridgeError::rejected("reconnect-profile", "saved profile no longer exists")
                })?;
            (
                profile.preferences().reconnect(),
                profile.preferences().restore_last_context(),
            )
        };
        let (action, delay_millis) =
            match reconnect_decision(preference, attempt, authentication_stopped) {
                ReconnectDecision::Manual => ("manual", None),
                ReconnectDecision::StopAuthentication => ("authentication_stopped", None),
                ReconnectDecision::RetryAfter { delay_millis } => ("retry", Some(delay_millis)),
                ReconnectDecision::Exhausted => ("exhausted", None),
            };
        Ok(BridgeReconnectPlan {
            action: action.into(),
            delay_millis,
            restore_last_context,
        })
    }

    fn reconnect_saved_session_inner(
        &self,
        old_session_bytes: Vec<u8>,
        password_override: Option<Vec<u8>>,
    ) -> Result<BridgeReconnectAttempt, BridgeError> {
        let old_session = session_from_bytes(&old_session_bytes)
            .map_err(|_| BridgeError::rejected("session-id", "invalid session id"))?;
        let profile_id = {
            let guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            guard
                .as_ref()
                .ok_or(BridgeError::RuntimeUnavailable)?
                .sessions
                .get(&old_session)
                .ok_or(BridgeError::UnknownSession)?
                .profile_id
        };
        let new_session =
            match self.open_profile_inner(profile_id.to_bytes().to_vec(), password_override) {
                Ok(session) => session,
                Err(BridgeError::Rejected { code, .. }) if code == "profile-password" => {
                    return Ok(BridgeReconnectAttempt {
                        state: "authentication_stopped".into(),
                        session_id: None,
                    });
                }
                Err(BridgeError::Rejected { code, .. }) if code == "connect" => {
                    return Ok(BridgeReconnectAttempt {
                        state: "retryable".into(),
                        session_id: None,
                    });
                }
                Err(error) => return Err(error),
            };
        if let Err(error) = self.disconnect_inner(old_session_bytes) {
            let _ = self.disconnect_inner(new_session.clone());
            return Err(error);
        }
        Ok(BridgeReconnectAttempt {
            state: "connected".into(),
            session_id: Some(new_session),
        })
    }

    fn check_session_health_inner(
        &self,
        session_id_bytes: Vec<u8>,
    ) -> Result<BridgeSessionHealth, BridgeError> {
        let session_id = session_from_bytes(&session_id_bytes)
            .map_err(|_| BridgeError::rejected("session-id", "invalid session id"))?;
        let (driver, expected_engine) = {
            let guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_ref().ok_or(BridgeError::RuntimeUnavailable)?;
            let registered = inner
                .sessions
                .get(&session_id)
                .ok_or(BridgeError::UnknownSession)?;
            let driver = inner
                .service
                .session(session_id)
                .ok_or(BridgeError::UnknownSession)?;
            (driver, registered.engine)
        };
        match self.runtime.block_on(driver.health())? {
            Ok(health) => {
                if health.engine() != expected_engine {
                    return Err(BridgeError::rejected(
                        "session-health-engine",
                        "health engine mismatch",
                    ));
                }
                Ok(BridgeSessionHealth {
                    state: if health.server_reachable() {
                        "healthy"
                    } else {
                        "unreachable"
                    }
                    .into(),
                    server_reachable: health.server_reachable(),
                    elapsed_millis: Some(health.elapsed_millis()),
                    authentication_stopped: false,
                })
            }
            Err(error) => Ok(BridgeSessionHealth {
                state: match error.class() {
                    AdapterFailureClass::Authentication => "authentication_stopped",
                    AdapterFailureClass::Timeout => "timeout",
                    AdapterFailureClass::Connection => "unreachable",
                    _ => "unhealthy",
                }
                .into(),
                server_reachable: false,
                elapsed_millis: None,
                authentication_stopped: error.class() == AdapterFailureClass::Authentication,
            }),
        }
    }

    /// Registers an already-constructed driver session (unit/conformance tests).
    ///
    /// Not exported to UniFFI — Rust-only seam for in-process tests.
    pub fn open_driver_session(
        &self,
        engine: Engine,
        session: Box<dyn DriverSession>,
    ) -> Result<Vec<u8>, BridgeError> {
        catch_entry(|| self.open_driver_session_inner(engine, session, None, None, None))
    }

    /// Registers a test driver under an existing saved-profile identity.
    /// Not exported to UniFFI.
    pub fn open_driver_session_for_profile(
        &self,
        profile_id: ProfileId,
        engine: Engine,
        session: Box<dyn DriverSession>,
    ) -> Result<Vec<u8>, BridgeError> {
        catch_entry(|| {
            self.open_driver_session_inner(engine, session, Some(profile_id), None, None)
        })
    }

    /// Inserts a minimal reviewed delete plan for the session (test/conformance only).
    ///
    /// Production Swift never builds plans; it receives token ids from Rust after
    /// Stage/Review commands. This seam proves handle consume-once/expiry without
    /// shipping plan bytes over UniFFI.
    ///
    /// `relation` is a PostgreSQL relation name in schema `public` (default
    /// `"users"` when empty). `locator_id` is the integer primary-key locator.
    pub fn insert_reviewed_probe(
        &self,
        session_id: Vec<u8>,
        issued_at_ms: u64,
        expires_at_ms: u64,
        now_ms: u64,
        relation: Option<String>,
        locator_id: Option<i64>,
    ) -> Result<Vec<u8>, BridgeError> {
        catch_entry(|| {
            self.insert_reviewed_probe_inner(
                session_id,
                issued_at_ms,
                expires_at_ms,
                now_ms,
                relation.unwrap_or_else(|| "users".into()),
                locator_id.unwrap_or(1),
            )
        })
    }

    #[must_use]
    pub fn new_for_test() -> Self {
        Self {
            runtime: RuntimeOwner::new(),
            inner: Mutex::new(None),
            postgres_tools: Arc::new(Mutex::new(BTreeMap::new())),
            redis_subscriptions: Arc::new(Mutex::new(BTreeMap::new())),
            csv_imports: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
}

impl TableRockBridge {
    fn ensure_runtime_inner(&self) -> Result<(), BridgeError> {
        self.runtime.ensure()?;
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        if guard.is_none() {
            *guard = Some(BridgeInner::new()?);
        }
        Ok(())
    }

    fn insert_reviewed_probe_inner(
        &self,
        session_id_bytes: Vec<u8>,
        issued_at_ms: u64,
        expires_at_ms: u64,
        now_ms: u64,
        relation: String,
        locator_id: i64,
    ) -> Result<Vec<u8>, BridgeError> {
        self.ensure_runtime_inner()?;
        let session_id = session_from_bytes(&session_id_bytes)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        let registered = inner
            .sessions
            .get(&session_id)
            .ok_or(BridgeError::UnknownSession)?;
        let scope = OperationScope::new(
            registered.profile_id,
            registered.session_id,
            registered.context_id,
        );
        let revision = registered.context_revision;
        let mutation_id = inner.ids.mutation();
        let token_id = inner.ids.review_token();
        let name = BoundedText::copy_from_str("id", ByteLimit::new(8))
            .map_err(|error| BridgeError::rejected("mutation-field", error.to_string()))?;
        let relation_text = BoundedText::copy_from_str(
            if relation.is_empty() {
                "users"
            } else {
                &relation
            },
            ByteLimit::new(128),
        )
        .map_err(|error| BridgeError::rejected("mutation-target", error.to_string()))?;
        let plan = MutationPlan::new(
            mutation_id,
            scope,
            revision,
            MutationTarget::PostgreSqlRelation {
                database: BoundedText::copy_from_str("postgres", ByteLimit::new(16))
                    .map_err(|error| BridgeError::rejected("mutation-target", error.to_string()))?,
                schema: BoundedText::copy_from_str("public", ByteLimit::new(8))
                    .map_err(|error| BridgeError::rejected("mutation-target", error.to_string()))?,
                relation: relation_text,
            },
            vec![MutationChange::DeleteRow {
                locator: vec![FieldValue::new(name, OwnedValue::signed(locator_id))],
            }],
            MutationPlanLimits::new(16, 16, 4096, 4096, 60_000)
                .map_err(|error| BridgeError::rejected("mutation-limits", error.to_string()))?,
        )
        .map_err(|error| BridgeError::rejected("mutation-plan", error.to_string()))?;
        let reviewed = plan
            .review(token_id, issued_at_ms, expires_at_ms)
            .map_err(|error| BridgeError::rejected("review", error.to_string()))?;
        inner
            .reviews
            .insert(reviewed, now_ms)
            .map_err(|error| BridgeError::rejected("review-insert", error.to_string()))?;
        Ok(review_token_bytes(token_id))
    }

    fn stage_csv_import_inner(
        &self,
        request: BridgeCsvImportRequest,
    ) -> Result<BridgeCsvImportReview, BridgeError> {
        let BridgeCsvImportRequest {
            session_id: session_id_bytes,
            catalog_node_id: catalog_node_id_bytes,
            path,
            mapped_columns,
            mapped_types,
            expected_fingerprint,
            now_ms,
        } = request;
        self.ensure_runtime_inner()?;
        let path_ref = Path::new(&path);
        if !path_ref.is_absolute() {
            return Err(BridgeError::rejected(
                "csv-import-path",
                "native CSV import path must be absolute",
            ));
        }
        let value_types = parse_csv_value_types(&mapped_types, mapped_columns.len())?;
        let session_id = session_from_bytes(&session_id_bytes)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let catalog_node_id = catalog_node_from_bytes(&catalog_node_id_bytes).map_err(|_| {
            BridgeError::rejected("bad-catalog-node-id", "catalog node id must be 16 bytes")
        })?;
        let expires_at_ms = now_ms
            .checked_add(60_000)
            .ok_or_else(|| BridgeError::rejected("csv-import-review", "review expiry overflow"))?;
        let (target, target_label, scope, revision, token_id) = {
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
            let registered = inner
                .sessions
                .get(&session_id)
                .ok_or(BridgeError::UnknownSession)?;
            let node = inner
                .catalog_nodes
                .get(&(session_id, catalog_node_id))
                .ok_or_else(|| {
                    BridgeError::rejected(
                        "unknown-catalog-node",
                        "catalog node is stale or unknown",
                    )
                })?;
            let parent = node
                .parent_id()
                .and_then(|id| inner.catalog_nodes.get(&(session_id, id)))
                .ok_or_else(|| BridgeError::rejected("catalog-parent", "object parent is stale"))?;
            let identifier = |value: &str| {
                BoundedText::copy_from_str(value, ByteLimit::new(256))
                    .map_err(|error| BridgeError::rejected("csv-import-target", error.to_string()))
            };
            let (target, label) = match (registered.engine, node.kind()) {
                (
                    Engine::PostgreSql,
                    CatalogNodeKind::PostgreSqlObject(
                        PostgreSqlObjectKind::Table
                        | PostgreSqlObjectKind::ForeignTable
                        | PostgreSqlObjectKind::PartitionedTable,
                    ),
                ) => (
                    MutationTarget::PostgreSqlRelation {
                        database: registered.database.clone(),
                        schema: identifier(parent.name())?,
                        relation: identifier(node.name())?,
                    },
                    format!("{}.{}", parent.name(), node.name()),
                ),
                (
                    Engine::ClickHouse,
                    CatalogNodeKind::ClickHouseObject(ClickHouseObjectKind::Table),
                ) => (
                    MutationTarget::ClickHouseTable {
                        database: identifier(parent.name())?,
                        table: identifier(node.name())?,
                    },
                    format!("{}.{}", parent.name(), node.name()),
                ),
                _ => {
                    return Err(BridgeError::rejected(
                        "csv-import-target",
                        "CSV import requires a cached writable table",
                    ));
                }
            };
            (
                target,
                label,
                OperationScope::new(
                    registered.profile_id,
                    registered.session_id,
                    registered.context_id,
                ),
                registered.context_revision,
                inner.ids.review_token(),
            )
        };

        let frozen_path = freeze_csv_source(path_ref, token_id)?;
        let staged = (|| -> Result<(u64, u64), BridgeError> {
            let mut validation_error = None;
            let summary =
                stream_csv_batches(&frozen_path, csv_stream_limits()?, |headers, rows, _| {
                    if mapped_columns.len() != headers.len()
                        || mapped_columns.iter().any(|column| column.is_empty())
                        || mapped_columns.iter().collect::<BTreeSet<_>>().len()
                            != mapped_columns.len()
                    {
                        validation_error = Some(BridgeError::rejected(
                            "csv-import-mapping",
                            "mapped columns must be non-empty, unique, and match CSV width",
                        ));
                        return false;
                    }
                    if !rows.is_empty() {
                        let table = CsvTable {
                            headers: mapped_columns.clone(),
                            rows: rows.to_vec(),
                        };
                        if let Err(error) = csv_to_typed_insert_changes(
                            &table,
                            &value_types,
                            CSV_IMPORT_MAX_CELL_BYTES as u64,
                        ) {
                            validation_error = Some(BridgeError::rejected(
                                "csv-import-values",
                                error.to_string(),
                            ));
                            return false;
                        }
                    }
                    true
                });
            if let Some(error) = validation_error {
                return Err(error);
            }
            let summary =
                summary.map_err(|error| BridgeError::rejected("csv-import", error.to_string()))?;
            let fingerprint = summary
                .sha256
                .map(hex_sha256)
                .ok_or_else(|| BridgeError::rejected("csv-import", "CSV hash is unavailable"))?;
            if fingerprint != expected_fingerprint {
                return Err(BridgeError::rejected(
                    "csv-import-changed",
                    "CSV file changed after preview; preview it again before review",
                ));
            }
            if summary.rows == 0 {
                return Err(BridgeError::rejected(
                    "csv-import-size",
                    "CSV import requires at least one data row",
                ));
            }
            Ok((summary.rows, summary.formula_like_cells))
        })();
        let (row_count, formula_like_cells) = match staged {
            Ok(value) => value,
            Err(error) => {
                let _ = std::fs::remove_file(&frozen_path);
                return Err(error);
            }
        };

        let insert_result = (|| -> Result<(), BridgeError> {
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
            let registered = inner
                .sessions
                .get(&session_id)
                .ok_or(BridgeError::UnknownSession)?;
            if registered.context_revision != revision {
                return Err(BridgeError::rejected(
                    "csv-import-stale",
                    "session context changed while staging CSV import",
                ));
            }
            expire_csv_import_reviews(&mut inner.csv_import_reviews, now_ms);
            if inner.csv_import_reviews.len() >= 256 {
                return Err(BridgeError::rejected(
                    "csv-import-review-limit",
                    "too many pending CSV import reviews",
                ));
            }
            inner.csv_import_reviews.insert(
                token_id,
                CsvImportReviewEntry {
                    session_id,
                    scope,
                    revision,
                    target,
                    frozen_path: frozen_path.clone(),
                    mapped_columns,
                    value_types,
                    row_count,
                    expires_at_ms,
                },
            );
            Ok(())
        })();
        if let Err(error) = insert_result {
            let _ = std::fs::remove_file(&frozen_path);
            return Err(error);
        }
        Ok(BridgeCsvImportReview {
            token_id: review_token_bytes(token_id),
            target: target_label,
            row_count: u32::try_from(row_count).unwrap_or(u32::MAX),
            column_count: u32::try_from(mapped_types.len()).unwrap_or(u32::MAX),
            formula_like_cells: u32::try_from(formula_like_cells).unwrap_or(u32::MAX),
            expires_at_ms,
        })
    }

    fn authorize_review_token_inner(
        &self,
        token_id_bytes: Vec<u8>,
        now_ms: u64,
        session_id_bytes: Vec<u8>,
        expected_revision: u64,
    ) -> Result<Vec<u8>, BridgeError> {
        // Kept in the stable bridge signature for compatibility. Authorization
        // always uses the registered session revision; caller claims are not
        // an authority source.
        let _ = expected_revision;
        self.ensure_runtime_inner()?;
        let token_id = review_token_from_bytes(&token_id_bytes).map_err(|_| {
            BridgeError::rejected("bad-token-id", "review token id must be 16 bytes")
        })?;
        let session_id = session_from_bytes(&session_id_bytes)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        let registered = inner
            .sessions
            .get(&session_id)
            .ok_or(BridgeError::UnknownSession)?;
        let scope = OperationScope::new(
            registered.profile_id,
            registered.session_id,
            registered.context_id,
        );
        let authorized = inner
            .reviews
            .authorize(token_id, now_ms, scope, registered.context_revision)
            .map_err(|error| BridgeError::rejected("authorize", error.to_string()))?;
        // Drop authorized plan immediately: bridge proves handle consume, not apply.
        drop(authorized);
        Ok(token_id_bytes)
    }

    fn revoke_review_token_inner(&self, token_id_bytes: Vec<u8>) -> Result<bool, BridgeError> {
        self.ensure_runtime_inner()?;
        let token_id = review_token_from_bytes(&token_id_bytes).map_err(|_| {
            BridgeError::rejected("bad-token-id", "review token id must be 16 bytes")
        })?;
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        if let Some(review) = inner.csv_import_reviews.remove(&token_id) {
            let _ = std::fs::remove_file(&review.frozen_path);
            return Ok(true);
        }
        Ok(inner.reviews.revoke(token_id))
    }

    fn apply_review_token_inner(
        &self,
        token_id_bytes: Vec<u8>,
        now_ms: u64,
        session_id_bytes: Vec<u8>,
        expected_revision: u64,
    ) -> Result<ApplyOutcome, BridgeError> {
        // See authorize_review_token_inner: session state owns the revision.
        let _ = expected_revision;
        self.ensure_runtime_inner()?;
        let token_id = review_token_from_bytes(&token_id_bytes).map_err(|_| {
            BridgeError::rejected("bad-token-id", "review token id must be 16 bytes")
        })?;
        let session_id = session_from_bytes(&session_id_bytes)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let (authorized, driver) = {
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
            if inner.csv_import_reviews.contains_key(&token_id) {
                return Err(BridgeError::rejected(
                    "csv-import-apply",
                    "CSV reviews require the progress-aware import apply path",
                ));
            }
            if !inner.accepting {
                return Err(BridgeError::ShuttingDown);
            }
            let registered = inner
                .sessions
                .get(&session_id)
                .ok_or(BridgeError::UnknownSession)?;
            let scope = OperationScope::new(
                registered.profile_id,
                registered.session_id,
                registered.context_id,
            );
            // Consume-once before any apply I/O.
            let authorized = inner
                .reviews
                .authorize(token_id, now_ms, scope, registered.context_revision)
                .map_err(|error| BridgeError::rejected("authorize", error.to_string()))?;
            let driver = inner
                .service
                .session(session_id)
                .ok_or(BridgeError::UnknownSession)?;
            (authorized, driver)
        };
        let outcome = self
            .runtime
            .block_on(driver.apply_authorized_mutation(authorized))?
            .map_err(|error| BridgeError::rejected("apply", error.to_string()))?;
        let mut applied = 0_u32;
        let mut conflict = 0_u32;
        let mut failed = 0_u32;
        for change in &outcome.changes {
            match change {
                tablerock_engine::MutationChangeOutcome::Applied { .. } => {
                    applied = applied.saturating_add(1);
                }
                tablerock_engine::MutationChangeOutcome::Conflict { .. } => {
                    conflict = conflict.saturating_add(1);
                }
                tablerock_engine::MutationChangeOutcome::Failed { .. } => {
                    failed = failed.saturating_add(1);
                }
            }
        }
        Ok(ApplyOutcome {
            transaction: format!("{:?}", outcome.transaction),
            change_count: u32::try_from(outcome.changes.len()).unwrap_or(u32::MAX),
            applied_count: applied,
            conflict_count: conflict,
            failed_count: failed,
        })
    }

    fn start_csv_import_apply_inner(
        &self,
        token_id_bytes: Vec<u8>,
        now_ms: u64,
        session_id_bytes: Vec<u8>,
    ) -> Result<Vec<u8>, BridgeError> {
        self.ensure_runtime_inner()?;
        let token_id = review_token_from_bytes(&token_id_bytes).map_err(|_| {
            BridgeError::rejected("bad-token-id", "review token id must be 16 bytes")
        })?;
        let session_id = session_from_bytes(&session_id_bytes)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let (review, driver, operation_id) = {
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
            if !inner.accepting {
                return Err(BridgeError::ShuttingDown);
            }
            let registered = inner
                .sessions
                .get(&session_id)
                .ok_or(BridgeError::UnknownSession)?;
            let scope = OperationScope::new(
                registered.profile_id,
                registered.session_id,
                registered.context_id,
            );
            let revision = registered.context_revision;
            let driver = inner
                .service
                .session(session_id)
                .ok_or(BridgeError::UnknownSession)?;
            let review = inner.csv_import_reviews.remove(&token_id).ok_or_else(|| {
                BridgeError::rejected(
                    "csv-import-review",
                    "review token is not a staged CSV import",
                )
            })?;
            if review.session_id != session_id
                || review.scope != scope
                || review.revision != revision
                || now_ms > review.expires_at_ms
            {
                let _ = std::fs::remove_file(&review.frozen_path);
                return Err(BridgeError::rejected(
                    "authorize",
                    "CSV import review expired or no longer matches the live session context",
                ));
            }
            (review, driver, inner.ids.operation())
        };
        let total_rows = review.row_count;

        let tasks = Arc::clone(&self.csv_imports);
        let progress_tasks = Arc::clone(&tasks);
        let control = MutationApplyControl::new(move |completed, total| {
            if let Ok(mut tasks) = progress_tasks.lock()
                && let Some(task) = tasks.get_mut(&operation_id)
            {
                task.completed_rows = completed.min(total);
                task.total_rows = total;
            }
        });
        {
            let mut registry = tasks.lock().map_err(|_| {
                BridgeError::rejected("csv-import-lock", "import task registry poisoned")
            })?;
            if registry.len() >= 64 {
                if let Some(terminal) = registry.iter().find_map(|(id, task)| {
                    (task.phase != "running" && task.phase != "cancel_requested").then_some(*id)
                }) {
                    registry.remove(&terminal);
                } else {
                    return Err(BridgeError::rejected(
                        "csv-import-limit",
                        "too many active CSV imports",
                    ));
                }
            }
            registry.insert(
                operation_id,
                CsvImportTask {
                    session_id,
                    control: control.clone(),
                    phase: "running".into(),
                    completed_rows: 0,
                    total_rows,
                    applied_rows: 0,
                    conflict_rows: 0,
                    failed_rows: 0,
                    errors: Vec::new(),
                    errors_truncated: false,
                    summary: "Applying reviewed import".into(),
                },
            );
        }
        self.runtime.spawn(async move {
            let result =
                apply_streamed_csv_import(driver, review, operation_id, control.clone()).await;
            let Ok(mut registry) = tasks.lock() else {
                return;
            };
            let Some(task) = registry.get_mut(&operation_id) else {
                return;
            };
            match result {
                Ok(outcome) => {
                    task.completed_rows = outcome.completed_rows;
                    task.applied_rows = outcome.applied_rows;
                    task.conflict_rows = outcome.conflict_rows;
                    task.failed_rows = outcome.failed_rows;
                    task.errors = outcome.errors;
                    task.errors_truncated = outcome.errors_truncated;
                    task.phase = outcome.phase;
                    task.summary = outcome.summary;
                }
                Err(_) => {
                    task.phase = if control.cancel_requested() {
                        "unknown_after_cancel"
                    } else {
                        "failed"
                    }
                    .into();
                    task.failed_rows = task.failed_rows.saturating_add(1);
                    task.errors
                        .push("Import dispatch failed; outcome may be unknown".into());
                    task.summary =
                        "Import apply failed without a confirmed terminal outcome".into();
                }
            }
        })?;
        Ok(operation_bytes(operation_id))
    }

    fn csv_import_progress_inner(
        &self,
        operation_id_bytes: Vec<u8>,
    ) -> Result<BridgeCsvImportProgress, BridgeError> {
        let operation_id = operation_from_bytes(&operation_id_bytes).map_err(|_| {
            BridgeError::rejected("bad-operation-id", "operation id must be 16 bytes")
        })?;
        let registry = self.csv_imports.lock().map_err(|_| {
            BridgeError::rejected("csv-import-lock", "import task registry poisoned")
        })?;
        let task = registry.get(&operation_id).ok_or_else(|| {
            BridgeError::rejected("csv-import-operation", "CSV import operation is unknown")
        })?;
        Ok(BridgeCsvImportProgress {
            operation_id: operation_bytes(operation_id),
            phase: task.phase.clone(),
            completed_rows: task.completed_rows,
            total_rows: task.total_rows,
            applied_rows: task.applied_rows,
            conflict_rows: task.conflict_rows,
            failed_rows: task.failed_rows,
            errors: task.errors.clone(),
            errors_truncated: task.errors_truncated,
            summary: task.summary.clone(),
        })
    }

    fn cancel_csv_import_inner(&self, operation_id_bytes: Vec<u8>) -> Result<bool, BridgeError> {
        let operation_id = operation_from_bytes(&operation_id_bytes).map_err(|_| {
            BridgeError::rejected("bad-operation-id", "operation id must be 16 bytes")
        })?;
        let mut registry = self.csv_imports.lock().map_err(|_| {
            BridgeError::rejected("csv-import-lock", "import task registry poisoned")
        })?;
        let task = registry.get_mut(&operation_id).ok_or_else(|| {
            BridgeError::rejected("csv-import-operation", "CSV import operation is unknown")
        })?;
        if task.phase != "running" && task.phase != "cancel_requested" {
            return Ok(false);
        }
        task.control.request_cancel();
        task.phase = "cancel_requested".into();
        task.summary = "Cancellation requested; waiting for the current row boundary".into();
        Ok(true)
    }

    fn dismiss_csv_import_inner(&self, operation_id_bytes: Vec<u8>) -> Result<bool, BridgeError> {
        let operation_id = operation_from_bytes(&operation_id_bytes).map_err(|_| {
            BridgeError::rejected("bad-operation-id", "operation id must be 16 bytes")
        })?;
        let mut registry = self.csv_imports.lock().map_err(|_| {
            BridgeError::rejected("csv-import-lock", "import task registry poisoned")
        })?;
        if registry
            .get(&operation_id)
            .is_some_and(|task| task.phase == "running" || task.phase == "cancel_requested")
        {
            return Ok(false);
        }
        Ok(registry.remove(&operation_id).is_some())
    }

    fn disconnect_inner(&self, session_id_bytes: Vec<u8>) -> Result<(), BridgeError> {
        self.ensure_runtime_inner()?;
        let session_id = session_from_bytes(&session_id_bytes)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        if self
            .postgres_tools
            .lock()
            .map_err(|_| {
                BridgeError::rejected("postgres-tool-lock", "tool registry mutex poisoned")
            })?
            .values()
            .any(|task| {
                task.session_id == session_id
                    && (task.phase == "running" || task.phase == "cancel_requested")
            })
        {
            return Err(BridgeError::rejected(
                "session-busy",
                "session still has an active PostgreSQL tool operation",
            ));
        }
        if self
            .redis_subscriptions
            .lock()
            .map_err(|_| {
                BridgeError::rejected(
                    "redis-subscription-lock",
                    "subscription registry mutex poisoned",
                )
            })?
            .values()
            .any(|task| {
                task.session_id == session_id
                    && (task.phase == "connecting"
                        || task.phase == "listening"
                        || task.phase == "cancel_requested")
            })
        {
            return Err(BridgeError::rejected(
                "session-busy",
                "session still has an active Redis subscription",
            ));
        }
        if self
            .csv_imports
            .lock()
            .map_err(|_| BridgeError::rejected("csv-import-lock", "import task registry poisoned"))?
            .values()
            .any(|task| {
                task.session_id == session_id
                    && (task.phase == "running" || task.phase == "cancel_requested")
            })
        {
            return Err(BridgeError::rejected(
                "session-busy",
                "session still has an active CSV import",
            ));
        }
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        self.runtime
            .block_on(inner.service.disconnect(session_id))?
            .map_err(|error| match error {
                tablerock_engine::EngineServiceError::SessionBusy => {
                    BridgeError::rejected("session-busy", "session still has active operations")
                }
                other => BridgeError::rejected("disconnect", other.to_string()),
            })?;
        let registered = inner
            .sessions
            .remove(&session_id)
            .ok_or(BridgeError::UnknownSession)?;
        inner
            .catalog_nodes
            .retain(|(cached_session, _), _| *cached_session != session_id);
        let scope = OperationScope::new(
            registered.profile_id,
            registered.session_id,
            registered.context_id,
        );
        inner
            .service
            .core_mut()
            .remove_scope(CommandScope::Context(scope))
            .map_err(|error| BridgeError::rejected("context-scope-cleanup", error.to_string()))?;
        inner
            .service
            .core_mut()
            .remove_scope(CommandScope::Session {
                profile_id: registered.profile_id,
                session_id: registered.session_id,
            })
            .map_err(|error| BridgeError::rejected("session-scope-cleanup", error.to_string()))?;
        if !inner
            .sessions
            .values()
            .any(|session| session.profile_id == registered.profile_id)
        {
            inner
                .service
                .core_mut()
                .remove_scope(CommandScope::Profile(registered.profile_id))
                .map_err(|error| {
                    BridgeError::rejected("profile-scope-cleanup", error.to_string())
                })?;
        }
        Ok(())
    }

    fn configure_persistence_inner(&self, path: String) -> Result<(), BridgeError> {
        self.ensure_runtime_inner()?;
        let actor = PersistenceActor::open(&path)
            .map_err(|error| BridgeError::rejected("persistence-open", error.to_string()))?;
        let history_retention = actor
            .history_retention()
            .map_err(|error| BridgeError::rejected("history-retention", error.to_string()))?;
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        if let Some(previous) = inner.persistence.take() {
            let _ = previous.shutdown();
        }
        inner.persistence = Some(actor);
        inner.history_retention = history_retention;
        Ok(())
    }

    fn open_profile_inner(
        &self,
        profile_id_bytes: Vec<u8>,
        password_override: Option<Vec<u8>>,
    ) -> Result<Vec<u8>, BridgeError> {
        self.ensure_runtime_inner()?;
        let profile_id =
            ProfileId::from_bytes(<[u8; 16]>::try_from(profile_id_bytes.as_slice()).map_err(
                |_| BridgeError::rejected("bad-profile-id", "profile id must be 16 bytes"),
            )?)
            .map_err(|error| BridgeError::rejected("bad-profile-id", error.to_string()))?;
        let params = {
            let guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_ref().ok_or(BridgeError::RuntimeUnavailable)?;
            let actor = inner.persistence.as_ref().ok_or_else(|| {
                BridgeError::rejected("persistence", "configure_persistence first")
            })?;
            let aggregate = actor
                .get_profile(profile_id)
                .map_err(|error| BridgeError::rejected("profile-load", error.to_string()))?
                .ok_or_else(|| BridgeError::rejected("profile-missing", "profile not found"))?;
            let connection = aggregate.connection();
            let props = connection.properties();
            let literal = |property: ProfileProperty| -> Option<String> {
                props.literal(property).map(str::to_owned)
            };
            let host = literal(ProfileProperty::Host).ok_or_else(|| {
                BridgeError::rejected("profile-host", "host literal required for bridge open")
            })?;
            let port = literal(ProfileProperty::Port)
                .ok_or_else(|| {
                    BridgeError::rejected("profile-port", "port literal required for bridge open")
                })?
                .parse::<u16>()
                .map_err(|_| BridgeError::rejected("profile-port", "invalid port"))?;
            let database = literal(ProfileProperty::DefaultContext).unwrap_or_default();
            let user = literal(ProfileProperty::Username).unwrap_or_default();
            struct OverridePrompt(Option<Vec<u8>>);
            impl SecretPromptPort for OverridePrompt {
                fn request(
                    &mut self,
                    _field: tablerock_core::SecretField,
                    _profile: &ProfileName,
                ) -> Result<ResolvedSecret, SecretResolutionError> {
                    self.0
                        .take()
                        .map(|value| ResolvedSecret::from_prompt(value, _field))
                        .transpose()?
                        .ok_or(SecretResolutionError::PromptFailed)
                }
            }
            struct OverrideKeychain(Option<Vec<u8>>);
            impl KeychainReadPort for OverrideKeychain {
                fn read(
                    &mut self,
                    _reference: &KeychainReference,
                ) -> Result<Vec<u8>, SecretResolutionError> {
                    self.0.take().ok_or(SecretResolutionError::KeychainFailed)
                }
            }
            let mut password_override = password_override;
            let password = if let Some(binding) = props.binding(ProfileProperty::Password) {
                let source = binding.secret_source().map(SecretSource::kind);
                let mut prompt = OverridePrompt(
                    matches!(source, Some(SecretSourceKind::PromptOnConnect))
                        .then(|| password_override.take())
                        .flatten(),
                );
                let mut keychain = OverrideKeychain(
                    matches!(source, Some(SecretSourceKind::Keychain(_)))
                        .then(|| password_override.take())
                        .flatten(),
                );
                let mut op = OpCliReader::default();
                let resolved = resolve_for_connect_with_ports(
                    binding,
                    connection.name(),
                    &mut prompt,
                    &mut op,
                    &mut keychain,
                )
                .map_err(|error| BridgeError::rejected("profile-password", error.to_string()))?;
                match resolved {
                    Some(secret) => std::str::from_utf8(secret.as_bytes())
                        .map(str::to_owned)
                        .map_err(|_| {
                            BridgeError::rejected(
                                "profile-password",
                                "password must be valid UTF-8",
                            )
                        })?,
                    None => String::new(),
                }
            } else {
                let bytes = password_override.ok_or_else(|| {
                    BridgeError::rejected(
                        "profile-password",
                        "prompt-on-connect profile requires a password override",
                    )
                })?;
                String::from_utf8(bytes).map_err(|_| {
                    BridgeError::rejected("profile-password", "password must be valid UTF-8")
                })?
            };
            let engine = match connection.engine() {
                Engine::PostgreSql => "postgresql",
                Engine::ClickHouse => "clickhouse",
                Engine::Redis => "redis",
            };
            OpenParams {
                engine: engine.into(),
                host,
                port,
                database,
                user,
                password,
                tls_mode: match connection.tls_policy() {
                    TlsPolicy::Disabled => "off",
                    TlsPolicy::VerifySystemRoots => "verify_ca",
                    TlsPolicy::VerifyCustomCa => "verify_full",
                    TlsPolicy::DangerousAcceptInvalidCertificate(_) => "off",
                }
                .into(),
            }
        };
        self.open_inner_for_profile(params, Some(profile_id))
    }

    fn get_profile_draft_inner(
        &self,
        profile_id_bytes: Vec<u8>,
    ) -> Result<BridgeProfileDraft, BridgeError> {
        let id = decode_profile_id(&profile_id_bytes)?;
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let actor = guard
            .as_ref()
            .ok_or(BridgeError::RuntimeUnavailable)?
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?;
        let aggregate = actor
            .get_profile(id)
            .map_err(|error| BridgeError::rejected("profile-read", error.to_string()))?
            .ok_or_else(|| BridgeError::rejected("profile-not-found", "profile not found"))?;
        profile_to_bridge_draft(&aggregate)
    }

    fn test_profile_inner(
        &self,
        profile_id: Vec<u8>,
        password_override: Option<Vec<u8>>,
    ) -> Result<BridgeConnectionTestReport, BridgeError> {
        let draft = self.get_profile_draft_inner(profile_id.clone())?;
        let session_bytes = self.open_profile_inner(profile_id, password_override)?;
        let session_id = session_from_bytes(&session_bytes)
            .map_err(|_| BridgeError::rejected("session-id", "invalid opened session id"))?;
        let driver = {
            let guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            guard
                .as_ref()
                .and_then(|inner| inner.service.session(session_id))
                .ok_or(BridgeError::UnknownSession)?
        };
        let described = self.runtime.block_on(driver.describe())?;
        let disconnect = self.disconnect_inner(session_bytes);
        let described =
            described.map_err(|error| BridgeError::rejected("profile-test", error.to_string()))?;
        disconnect?;
        Ok(BridgeConnectionTestReport {
            identity: described.identity().to_owned(),
            tls_outcome: if draft.tls_mode == "off" {
                "disabled"
            } else {
                "verified"
            }
            .to_owned(),
            elapsed_millis: described.elapsed_millis(),
        })
    }

    fn save_profile_inner(&self, draft: BridgeProfileDraft) -> Result<Vec<u8>, BridgeError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        let actor = inner
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?;
        let existing = draft
            .id_bytes
            .as_deref()
            .map(decode_profile_id)
            .transpose()?
            .map(|id| {
                actor
                    .get_profile(id)
                    .map_err(|error| BridgeError::rejected("profile-read", error.to_string()))?
                    .ok_or_else(|| BridgeError::rejected("profile-not-found", "profile not found"))
            })
            .transpose()?;
        let id = existing
            .as_ref()
            .map(|profile| profile.connection().id())
            .unwrap_or_else(|| inner.ids.profile());
        let aggregate = bridge_draft_to_profile(&draft, id, existing.as_ref())?;
        if existing.is_some() {
            actor
                .replace_profile(
                    Revision::from_wire_u64(draft.revision),
                    aggregate.persistable().expect("saved profile"),
                )
                .map_err(|error| BridgeError::rejected("profile-save", error.to_string()))?;
        } else {
            actor
                .create_profile(aggregate.persistable().expect("saved profile"))
                .map_err(|error| BridgeError::rejected("profile-save", error.to_string()))?;
        }
        Ok(id.to_bytes().to_vec())
    }

    fn delete_profile_inner(
        &self,
        profile_id_bytes: Vec<u8>,
        expected_revision: u64,
    ) -> Result<(), BridgeError> {
        let id = decode_profile_id(&profile_id_bytes)?;
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let actor = guard
            .as_ref()
            .ok_or(BridgeError::RuntimeUnavailable)?
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?;
        actor
            .delete_profile(id, Revision::from_wire_u64(expected_revision))
            .map_err(|error| BridgeError::rejected("profile-delete", error.to_string()))
    }

    fn list_profiles_inner(
        &self,
        search: Option<String>,
    ) -> Result<Vec<BridgeProfileItem>, BridgeError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_ref().ok_or(BridgeError::RuntimeUnavailable)?;
        let actor = inner
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?;
        let search = search
            .filter(|value| !value.trim().is_empty())
            .map(|value| {
                ProfileSearchTerm::new(
                    BoundedText::copy_from_str(&value, ByteLimit::new(256)).map_err(|error| {
                        BridgeError::rejected("profile-search", error.to_string())
                    })?,
                )
                .map_err(|error| BridgeError::rejected("profile-search", error.to_string()))
            })
            .transpose()?;
        let filter = ProfileListFilter::new(None, None).with_search(search);
        let connected_profiles = inner
            .sessions
            .values()
            .map(|session| session.profile_id)
            .collect::<BTreeSet<_>>();
        let mut after = None;
        let mut items = Vec::new();
        loop {
            let request = ProfileListRequest::new(filter.clone(), after, 100).map_err(|error| {
                BridgeError::rejected("profile-list-request", error.to_string())
            })?;
            let page = actor
                .list_profiles(request)
                .map_err(|error| BridgeError::rejected("profile-list", error.to_string()))?;
            items.extend(
                page.items()
                    .iter()
                    .map(|item| bridge_profile_item(item, connected_profiles.contains(&item.id()))),
            );
            if items.len() > ProfileListRequest::MAX_SEARCH_CANDIDATES {
                return Err(BridgeError::rejected(
                    "profile-list",
                    "profile list exceeds bounded capacity",
                ));
            }
            let Some(next) = page.next() else { break };
            after = Some(next);
        }
        Ok(items)
    }

    fn list_profile_groups_inner(&self) -> Result<Vec<BridgeProfileGroup>, BridgeError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let actor = guard
            .as_ref()
            .ok_or(BridgeError::RuntimeUnavailable)?
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?;
        actor
            .list_group_settings()
            .map(|groups| {
                groups
                    .into_iter()
                    .map(|group| BridgeProfileGroup {
                        name: group.name,
                        alphabetical: group.alphabetical,
                    })
                    .collect()
            })
            .map_err(|error| BridgeError::rejected("profile-groups", error.to_string()))
    }

    fn list_history_inner(
        &self,
        search: Option<String>,
        limit: u32,
    ) -> Result<Vec<BridgeHistoryItem>, BridgeError> {
        if limit == 0 || limit > 500 {
            return Err(BridgeError::rejected(
                "history-limit",
                "history limit must be between 1 and 500",
            ));
        }
        let search = search
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if search.as_ref().is_some_and(|value| value.len() > 256) {
            return Err(BridgeError::rejected(
                "history-search",
                "history search exceeds 256 bytes",
            ));
        }
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let actor = guard
            .as_ref()
            .ok_or(BridgeError::RuntimeUnavailable)?
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?;
        actor
            .list_history(search, limit)
            .map(|entries| {
                entries
                    .into_iter()
                    .map(|entry| BridgeHistoryItem {
                        history_id: entry.history_id,
                        engine: engine_label(entry.engine).into(),
                        database_name: entry.database_name,
                        schema_name: entry.schema_name,
                        statement_text: entry.statement_text,
                        outcome: entry.outcome.as_str().into(),
                        created_at: entry.created_at,
                    })
                    .collect()
            })
            .map_err(|error| BridgeError::rejected("history-list", error.to_string()))
    }

    fn set_history_retention_inner(&self, retention: String) -> Result<(), BridgeError> {
        let retention = match retention.as_str() {
            "full" => HistoryRetention::Full,
            "metadata_only" => HistoryRetention::MetadataOnly,
            "private" => HistoryRetention::Private,
            _ => {
                return Err(BridgeError::rejected(
                    "history-retention",
                    "unknown history retention",
                ));
            }
        };
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        inner
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?
            .set_history_retention(retention)
            .map_err(|error| BridgeError::rejected("history-retention", error.to_string()))?;
        inner.history_retention = retention;
        Ok(())
    }

    fn history_retention_inner(&self) -> Result<String, BridgeError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let retention = guard
            .as_ref()
            .ok_or(BridgeError::RuntimeUnavailable)?
            .history_retention;
        Ok(match retention {
            HistoryRetention::Full => "full",
            HistoryRetention::MetadataOnly => "metadata_only",
            HistoryRetention::Private => "private",
        }
        .into())
    }

    fn list_saved_queries_inner(
        &self,
        engine: Option<String>,
        search: Option<String>,
    ) -> Result<Vec<BridgeSavedQueryItem>, BridgeError> {
        let engine = engine.as_deref().map(parse_engine).transpose()?;
        let search = search
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty());
        if search.as_ref().is_some_and(|value| value.len() > 256) {
            return Err(BridgeError::rejected(
                "saved-query-search",
                "saved-query search exceeds 256 bytes",
            ));
        }
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let actor = guard
            .as_ref()
            .ok_or(BridgeError::RuntimeUnavailable)?
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?;
        let queries = actor
            .list_saved_queries(engine)
            .map_err(|error| BridgeError::rejected("saved-query-list", error.to_string()))?;
        if queries.len() > 1_000 {
            return Err(BridgeError::rejected(
                "saved-query-list",
                "saved-query list exceeds bounded capacity",
            ));
        }
        Ok(queries
            .into_iter()
            .filter(|query| {
                search.as_ref().is_none_or(|term| {
                    query.name.to_ascii_lowercase().contains(term)
                        || query.statement_text.to_ascii_lowercase().contains(term)
                })
            })
            .map(|query| BridgeSavedQueryItem {
                query_id: query.query_id,
                name: query.name,
                engine: engine_label(query.engine).into(),
                statement_text: query.statement_text,
                updated_at: query.updated_at,
            })
            .collect())
    }

    fn save_query_inner(
        &self,
        name: String,
        engine: String,
        statement_text: String,
    ) -> Result<i64, BridgeError> {
        let engine = parse_engine(&engine)?;
        if name.trim().is_empty() || name.len() > 128 {
            return Err(BridgeError::rejected(
                "saved-query-name",
                "saved-query name must be 1 to 128 bytes",
            ));
        }
        if statement_text.trim().is_empty() || statement_text.len() > 1_048_576 {
            return Err(BridgeError::rejected(
                "saved-query-statement",
                "saved-query SQL must be 1 to 1048576 bytes",
            ));
        }
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        guard
            .as_ref()
            .ok_or(BridgeError::RuntimeUnavailable)?
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?
            .upsert_saved_query(SavedQueryUpsert {
                name,
                engine,
                statement_text,
            })
            .map_err(|error| BridgeError::rejected("saved-query-save", error.to_string()))
    }

    fn delete_saved_query_inner(&self, query_id: i64) -> Result<bool, BridgeError> {
        if query_id <= 0 {
            return Err(BridgeError::rejected(
                "saved-query-id",
                "saved-query id must be positive",
            ));
        }
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        guard
            .as_ref()
            .ok_or(BridgeError::RuntimeUnavailable)?
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?
            .delete_saved_query(query_id)
            .map_err(|error| BridgeError::rejected("saved-query-delete", error.to_string()))
    }

    fn list_catalog_filter_presets_inner(
        &self,
        session_id: Vec<u8>,
        catalog_node_id: Vec<u8>,
    ) -> Result<Vec<BridgeSavedFilterPreset>, BridgeError> {
        let session_id = session_from_bytes(&session_id)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let catalog_node_id = catalog_node_from_bytes(&catalog_node_id).map_err(|_| {
            BridgeError::rejected("bad-catalog-node-id", "catalog node id must be 16 bytes")
        })?;
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_ref().ok_or(BridgeError::RuntimeUnavailable)?;
        let registered = inner
            .sessions
            .get(&session_id)
            .ok_or(BridgeError::UnknownSession)?;
        let node = inner
            .catalog_nodes
            .get(&(session_id, catalog_node_id))
            .ok_or_else(|| {
                BridgeError::rejected("unknown-catalog-node", "catalog node is stale")
            })?;
        let parent = node
            .parent_id()
            .and_then(|parent| inner.catalog_nodes.get(&(session_id, parent)))
            .ok_or_else(|| BridgeError::rejected("catalog-parent", "object parent is stale"))?;
        let actor = inner
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?;
        let Some(record) = actor
            .get_saved_filter_library(registered.profile_id)
            .map_err(|error| BridgeError::rejected("saved-filter-list", error.to_string()))?
        else {
            return Ok(Vec::new());
        };
        let library = SavedFilterLibrary::from_json(&record.library_json).ok_or_else(|| {
            BridgeError::rejected("saved-filter-library", "saved filter library is invalid")
        })?;
        let presets = library
            .presets
            .into_iter()
            .filter(|preset| preset.schema == parent.name() && preset.table == node.name())
            .map(bridge_saved_filter_preset)
            .collect::<Vec<_>>();
        if presets.len() > 256 {
            return Err(BridgeError::rejected(
                "saved-filter-list",
                "saved filter preset list exceeds 256 entries",
            ));
        }
        Ok(presets)
    }

    fn save_catalog_filter_preset_inner(
        &self,
        session_id: Vec<u8>,
        catalog_node_id: Vec<u8>,
        preset: BridgeSavedFilterPreset,
    ) -> Result<(), BridgeError> {
        validate_bridge_saved_filter_preset(&preset)?;
        let session_id = session_from_bytes(&session_id)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let catalog_node_id = catalog_node_from_bytes(&catalog_node_id).map_err(|_| {
            BridgeError::rejected("bad-catalog-node-id", "catalog node id must be 16 bytes")
        })?;
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_ref().ok_or(BridgeError::RuntimeUnavailable)?;
        let registered = inner
            .sessions
            .get(&session_id)
            .ok_or(BridgeError::UnknownSession)?;
        let node = inner
            .catalog_nodes
            .get(&(session_id, catalog_node_id))
            .ok_or_else(|| {
                BridgeError::rejected("unknown-catalog-node", "catalog node is stale")
            })?;
        let parent = node
            .parent_id()
            .and_then(|parent| inner.catalog_nodes.get(&(session_id, parent)))
            .ok_or_else(|| BridgeError::rejected("catalog-parent", "object parent is stale"))?;
        let actor = inner
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?;
        let record = actor
            .get_saved_filter_library(registered.profile_id)
            .map_err(|error| BridgeError::rejected("saved-filter-read", error.to_string()))?;
        let mut library = match record {
            Some(record) => {
                SavedFilterLibrary::from_json(&record.library_json).ok_or_else(|| {
                    BridgeError::rejected("saved-filter-library", "saved filter library is invalid")
                })?
            }
            None => SavedFilterLibrary::default(),
        };
        library.upsert(SavedFilterPreset {
            name: preset.name,
            schema: parent.name().to_owned(),
            table: node.name().to_owned(),
            filters: preset
                .filters
                .into_iter()
                .map(|filter| SavedFilterCondition {
                    column: filter.column,
                    operator: filter.operator,
                    value: filter.value,
                })
                .collect(),
            raw_where: preset.raw_where,
        });
        actor
            .put_saved_filter_library(registered.profile_id, library.to_json())
            .map_err(|error| BridgeError::rejected("saved-filter-save", error.to_string()))
    }

    fn put_session_intent_inner(
        &self,
        profile_id: Vec<u8>,
        intent: BridgeSessionIntent,
    ) -> Result<(), BridgeError> {
        validate_session_intent(&intent)?;
        let profile_id = decode_profile_id(&profile_id)?;
        let intent_json = encode_session_intent(intent)?;
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        guard
            .as_ref()
            .ok_or(BridgeError::RuntimeUnavailable)?
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?
            .put_session_intent(profile_id, intent_json)
            .map_err(|error| BridgeError::rejected("session-intent-save", error.to_string()))
    }

    fn get_session_intent_inner(
        &self,
        profile_id: Vec<u8>,
    ) -> Result<Option<BridgeSessionIntent>, BridgeError> {
        let profile_id = decode_profile_id(&profile_id)?;
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let record = guard
            .as_ref()
            .ok_or(BridgeError::RuntimeUnavailable)?
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?
            .get_session_intent(profile_id)
            .map_err(|error| BridgeError::rejected("session-intent-load", error.to_string()))?;
        record
            .map(|record| decode_session_intent(&record.intent_json))
            .transpose()
    }

    fn delete_session_intent_inner(&self, profile_id: Vec<u8>) -> Result<(), BridgeError> {
        let profile_id = decode_profile_id(&profile_id)?;
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        guard
            .as_ref()
            .ok_or(BridgeError::RuntimeUnavailable)?
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?
            .delete_session_intent(profile_id)
            .map_err(|error| BridgeError::rejected("session-intent-delete", error.to_string()))
    }

    fn put_native_window_intent_inner(
        &self,
        window_id: String,
        profile_id: Vec<u8>,
        intent: BridgeSessionIntent,
    ) -> Result<(), BridgeError> {
        validate_session_intent(&intent)?;
        let profile_id = decode_profile_id(&profile_id)?;
        let intent_json = encode_session_intent(intent)?;
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        guard
            .as_ref()
            .ok_or(BridgeError::RuntimeUnavailable)?
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?
            .put_native_window_intent(window_id, profile_id, intent_json)
            .map_err(|error| BridgeError::rejected("native-window-intent-save", error.to_string()))
    }

    fn get_native_window_intent_inner(
        &self,
        window_id: String,
    ) -> Result<Option<BridgeNativeWindowIntent>, BridgeError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let record = guard
            .as_ref()
            .ok_or(BridgeError::RuntimeUnavailable)?
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?
            .get_native_window_intent(window_id)
            .map_err(|error| {
                BridgeError::rejected("native-window-intent-load", error.to_string())
            })?;
        record
            .map(|record| {
                Ok(BridgeNativeWindowIntent {
                    profile_id: record.profile_id.to_bytes().to_vec(),
                    intent: decode_session_intent(&record.intent_json)?,
                })
            })
            .transpose()
    }

    fn delete_native_window_intent_inner(&self, window_id: String) -> Result<(), BridgeError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        guard
            .as_ref()
            .ok_or(BridgeError::RuntimeUnavailable)?
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?
            .delete_native_window_intent(window_id)
            .map_err(|error| {
                BridgeError::rejected("native-window-intent-delete", error.to_string())
            })
    }

    fn create_profile_group_inner(&self, name: String) -> Result<(), BridgeError> {
        let name = validate_bridge_group_name(&name)?;
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let actor = guard
            .as_ref()
            .ok_or(BridgeError::RuntimeUnavailable)?
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?;
        actor
            .create_group(&name)
            .map_err(|error| BridgeError::rejected("profile-group-create", error.to_string()))
    }

    fn rename_profile_group_inner(
        &self,
        old_name: String,
        new_name: String,
    ) -> Result<u32, BridgeError> {
        let old_name = validate_bridge_group_name(&old_name)?;
        let new_name = validate_bridge_group_name(&new_name)?;
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let actor = guard
            .as_ref()
            .ok_or(BridgeError::RuntimeUnavailable)?
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?;
        actor
            .rename_group(&old_name, &new_name)
            .map_err(|error| BridgeError::rejected("profile-group-rename", error.to_string()))
    }

    fn delete_profile_group_inner(&self, name: String) -> Result<u32, BridgeError> {
        let name = validate_bridge_group_name(&name)?;
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let actor = guard
            .as_ref()
            .ok_or(BridgeError::RuntimeUnavailable)?
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?;
        actor
            .delete_group(&name)
            .map_err(|error| BridgeError::rejected("profile-group-delete", error.to_string()))
    }

    fn set_profile_group_alphabetical_inner(
        &self,
        name: String,
        alphabetical: bool,
    ) -> Result<(), BridgeError> {
        let name = validate_bridge_group_name(&name)?;
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let actor = guard
            .as_ref()
            .ok_or(BridgeError::RuntimeUnavailable)?
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?;
        actor
            .set_group_alphabetical(&name, alphabetical)
            .map_err(|error| BridgeError::rejected("profile-group-order", error.to_string()))
    }

    fn set_profile_favorite_inner(
        &self,
        profile_id: Vec<u8>,
        expected_revision: u64,
        favorite: bool,
    ) -> Result<(), BridgeError> {
        let id = decode_profile_id(&profile_id)?;
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let actor = guard
            .as_ref()
            .ok_or(BridgeError::RuntimeUnavailable)?
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?;
        actor
            .set_profile_favorite(id, Revision::from_wire_u64(expected_revision), favorite)
            .map_err(|error| BridgeError::rejected("profile-favorite", error.to_string()))
    }

    fn reorder_profiles_inner(
        &self,
        group: Option<String>,
        ordered: Vec<BridgeProfileOrderItem>,
    ) -> Result<(), BridgeError> {
        if ordered.len() > ProfileListRequest::MAX_SEARCH_CANDIDATES {
            return Err(BridgeError::rejected(
                "profile-order",
                "profile order exceeds bounded capacity",
            ));
        }
        let group = group
            .as_deref()
            .map(validate_bridge_group_name)
            .transpose()?;
        let updates = ordered
            .into_iter()
            .map(|item| {
                Ok(ProfileOrderUpdate {
                    id: decode_profile_id(&item.id_bytes)?,
                    expected_revision: Revision::from_wire_u64(item.expected_revision),
                })
            })
            .collect::<Result<Vec<_>, BridgeError>>()?;
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let actor = guard
            .as_ref()
            .ok_or(BridgeError::RuntimeUnavailable)?
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?;
        actor
            .reorder_profiles(group.as_deref(), updates)
            .map_err(|error| BridgeError::rejected("profile-order", error.to_string()))
    }

    fn refresh_catalog_inner(
        &self,
        session_id_bytes: Vec<u8>,
        parent_node_id_bytes: Option<Vec<u8>>,
    ) -> Result<Vec<BridgeCatalogNode>, BridgeError> {
        self.ensure_runtime_inner()?;
        let session_id = session_from_bytes(&session_id_bytes)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let parent_id = parent_node_id_bytes
            .as_deref()
            .map(catalog_node_from_bytes)
            .transpose()
            .map_err(|_| {
                BridgeError::rejected("bad-catalog-node-id", "catalog node id must be 16 bytes")
            })?;
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        let registered = inner
            .sessions
            .get(&session_id)
            .ok_or(BridgeError::UnknownSession)?
            .clone();
        let limits = default_page_limits();
        let (request, parent, expected_level) = match parent_id {
            None => (
                match registered.engine {
                    Engine::PostgreSql => CatalogRequest::PostgreSqlDatabases { limits },
                    Engine::ClickHouse => CatalogRequest::ClickHouseDatabases { limits },
                    Engine::Redis => CatalogRequest::RedisLogicalDatabases { limits },
                },
                None,
                match registered.engine {
                    Engine::PostgreSql => CatalogExpectedLevel::PostgreSqlDatabase,
                    Engine::ClickHouse => CatalogExpectedLevel::ClickHouseDatabase,
                    Engine::Redis => CatalogExpectedLevel::RedisLogicalDatabase,
                },
            ),
            Some(parent_id) => {
                let node = inner
                    .catalog_nodes
                    .get(&(session_id, parent_id))
                    .ok_or_else(|| {
                        BridgeError::rejected(
                            "unknown-catalog-node",
                            "catalog node is stale or unknown",
                        )
                    })?;
                let (request, expected_level) = match node.kind() {
                    CatalogNodeKind::PostgreSqlDatabase => (
                        CatalogRequest::PostgreSqlSchemas {
                            database: bounded_catalog_name(node.name())?,
                            limits,
                        },
                        CatalogExpectedLevel::PostgreSqlSchema,
                    ),
                    CatalogNodeKind::PostgreSqlSchema => {
                        let database_id = node.parent_id().ok_or_else(|| {
                            BridgeError::rejected("catalog-parent", "schema has no database parent")
                        })?;
                        let database = inner
                            .catalog_nodes
                            .get(&(session_id, database_id))
                            .ok_or_else(|| {
                                BridgeError::rejected("catalog-parent", "database parent is stale")
                            })?;
                        (
                            CatalogRequest::PostgreSqlRelations {
                                database: bounded_catalog_name(database.name())?,
                                schema: bounded_catalog_name(node.name())?,
                                limits,
                            },
                            CatalogExpectedLevel::PostgreSqlObject,
                        )
                    }
                    CatalogNodeKind::ClickHouseDatabase => (
                        CatalogRequest::ClickHouseObjects {
                            database: bounded_catalog_name(node.name())?,
                            limits,
                        },
                        CatalogExpectedLevel::ClickHouseObject,
                    ),
                    CatalogNodeKind::RedisLogicalDatabase => {
                        let selected = node.name().strip_prefix("db").unwrap_or(node.name());
                        if selected != registered.database.as_str() {
                            return Err(BridgeError::rejected(
                                "redis-database-context",
                                "reconnect with this Redis logical database before listing keys",
                            ));
                        }
                        (
                            CatalogRequest::RedisKeys { limits },
                            CatalogExpectedLevel::RedisKey,
                        )
                    }
                    _ => {
                        return Err(BridgeError::rejected(
                            "catalog-leaf",
                            "catalog node has no supported child request",
                        ));
                    }
                };
                (request, Some((parent_id, node.depth())), expected_level)
            }
        };
        let driver = inner
            .service
            .session(session_id)
            .ok_or(BridgeError::UnknownSession)?;
        let subtree = self.runtime.block_on(async {
            driver
                .catalog(request)
                .await
                .map_err(|error| BridgeError::rejected("catalog-refresh", error.to_string()))
        })??;
        if subtree.engine() != registered.engine {
            return Err(BridgeError::rejected(
                "catalog-engine",
                "catalog subtree engine mismatch",
            ));
        }
        let (parent_id, depth) = parent
            .map(|(id, parent_depth)| (Some(id), parent_depth.saturating_add(1)))
            .unwrap_or((None, 0));
        let seeds = subtree.into_nodes();
        if seeds.len() > 1_000
            || seeds.iter().map(|seed| seed.name().len()).sum::<usize>() > 100_000
        {
            return Err(BridgeError::rejected(
                "catalog-bounds",
                "catalog subtree exceeds bridge bounds",
            ));
        }
        if seeds.iter().any(|seed| {
            !expected_level.accepts(seed.kind())
                || matches!(seed.children(), CatalogChildrenState::Failed)
        }) {
            return Err(BridgeError::rejected(
                "catalog-shape",
                "catalog adapter returned an invalid child kind",
            ));
        }
        let nodes = seeds
            .into_iter()
            .map(|seed| {
                let id = inner.ids.catalog_node();
                let kind = seed.kind();
                let children = seed.children();
                let name = seed.clone().into_name();
                let engine_type = seed.take_engine_type();
                CatalogNode::new(id, parent_id, depth, kind, name, engine_type, children)
            })
            .collect::<Vec<_>>();
        if parent_id.is_none() {
            inner
                .catalog_nodes
                .retain(|(cached_session, _), _| *cached_session != session_id);
        } else if let Some(parent_id) = parent_id {
            let mut stale = BTreeSet::new();
            let mut frontier = BTreeSet::from([parent_id]);
            loop {
                let children = inner
                    .catalog_nodes
                    .iter()
                    .filter_map(|((cached_session, id), node)| {
                        (*cached_session == session_id
                            && node
                                .parent_id()
                                .is_some_and(|parent| frontier.contains(&parent)))
                        .then_some(*id)
                    })
                    .collect::<BTreeSet<_>>();
                let fresh = children
                    .difference(&stale)
                    .copied()
                    .collect::<BTreeSet<_>>();
                if fresh.is_empty() {
                    break;
                }
                stale.extend(&fresh);
                frontier = fresh;
            }
            inner.catalog_nodes.retain(|(cached_session, id), _| {
                *cached_session != session_id || !stale.contains(id)
            });
        }
        for node in &nodes {
            inner
                .catalog_nodes
                .insert((session_id, node.id()), node.clone());
        }
        Ok(nodes.iter().map(bridge_catalog_node).collect())
    }

    fn submit_catalog_browse_inner(
        &self,
        session_id_bytes: Vec<u8>,
        catalog_node_id_bytes: Vec<u8>,
        sort: Vec<BridgeBrowseSort>,
        filters: Vec<BridgeBrowseFilter>,
        raw_where: Option<String>,
        row_count: u32,
    ) -> Result<Vec<u8>, BridgeError> {
        if !(1..=1_000).contains(&row_count) {
            return Err(BridgeError::rejected(
                "catalog-browse-bounds",
                "catalog browse row count must be 1 to 1000",
            ));
        }
        let session_id = session_from_bytes(&session_id_bytes)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let catalog_node_id = catalog_node_from_bytes(&catalog_node_id_bytes).map_err(|_| {
            BridgeError::rejected("bad-catalog-node-id", "catalog node id must be 16 bytes")
        })?;
        let (rendered, copy_identity) = {
            let guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_ref().ok_or(BridgeError::RuntimeUnavailable)?;
            let registered = inner
                .sessions
                .get(&session_id)
                .ok_or(BridgeError::UnknownSession)?;
            let node = inner
                .catalog_nodes
                .get(&(session_id, catalog_node_id))
                .ok_or_else(|| {
                    BridgeError::rejected(
                        "unknown-catalog-node",
                        "catalog node is stale or unknown",
                    )
                })?;
            let parent_id = node.parent_id().ok_or_else(|| {
                BridgeError::rejected("catalog-browse-kind", "catalog object has no parent")
            })?;
            let parent = inner
                .catalog_nodes
                .get(&(session_id, parent_id))
                .ok_or_else(|| BridgeError::rejected("catalog-parent", "object parent is stale"))?;
            let supported = matches!(
                (registered.engine, node.kind()),
                (
                    Engine::PostgreSql,
                    CatalogNodeKind::PostgreSqlObject(
                        PostgreSqlObjectKind::Table
                            | PostgreSqlObjectKind::View
                            | PostgreSqlObjectKind::MaterializedView
                            | PostgreSqlObjectKind::ForeignTable
                            | PostgreSqlObjectKind::PartitionedTable
                            | PostgreSqlObjectKind::Sequence
                    )
                ) | (Engine::ClickHouse, CatalogNodeKind::ClickHouseObject(_))
            );
            if !supported {
                return Err(BridgeError::rejected(
                    "catalog-browse-kind",
                    "catalog node is not a browsable table-like object",
                ));
            }
            let insertable = matches!(
                (registered.engine, node.kind()),
                (
                    Engine::PostgreSql,
                    CatalogNodeKind::PostgreSqlObject(
                        PostgreSqlObjectKind::Table
                            | PostgreSqlObjectKind::ForeignTable
                            | PostgreSqlObjectKind::PartitionedTable
                    )
                ) | (
                    Engine::ClickHouse,
                    CatalogNodeKind::ClickHouseObject(ClickHouseObjectKind::Table)
                )
            );
            let schema = parent.name().to_owned();
            let table = node.name().to_owned();
            if sort.len() > 16 {
                return Err(BridgeError::rejected(
                    "catalog-browse-sort",
                    "at most 16 sort keys are allowed",
                ));
            }
            let mut seen = BTreeSet::new();
            let sort = sort
                .into_iter()
                .map(|key| {
                    if key.column.len() > MAX_BROWSE_IDENTIFIER_BYTES {
                        return Err(BridgeError::rejected(
                            "catalog-browse-sort",
                            "sort column must be at most 1024 bytes",
                        ));
                    }
                    if !seen.insert(key.column.clone()) {
                        return Err(BridgeError::rejected(
                            "catalog-browse-sort",
                            "sort columns must be unique",
                        ));
                    }
                    let direction = match key.direction.as_str() {
                        "asc" => SortDirection::Asc,
                        "desc" => SortDirection::Desc,
                        _ => {
                            return Err(BridgeError::rejected(
                                "catalog-browse-sort",
                                "sort direction must be asc or desc",
                            ));
                        }
                    };
                    Ok(SortKey {
                        column: key.column,
                        direction,
                    })
                })
                .collect::<Result<Vec<_>, BridgeError>>()?;
            if filters.len() > 32 {
                return Err(BridgeError::rejected(
                    "catalog-browse-filter",
                    "at most 32 filters are allowed",
                ));
            }
            let filters = filters
                .into_iter()
                .map(|filter| {
                    if filter.column.len() > MAX_BROWSE_IDENTIFIER_BYTES {
                        return Err(BridgeError::rejected(
                            "catalog-browse-filter",
                            "filter column must be at most 1024 bytes",
                        ));
                    }
                    if filter
                        .value
                        .as_ref()
                        .is_some_and(|value| value.len() > MAX_BROWSE_VALUE_BYTES)
                    {
                        return Err(BridgeError::rejected(
                            "catalog-browse-filter",
                            "filter value must be at most 65536 bytes",
                        ));
                    }
                    let operator = match filter.operator.as_str() {
                        "eq" => FilterOperator::Eq,
                        "ne" => FilterOperator::Ne,
                        "lt" => FilterOperator::Lt,
                        "le" => FilterOperator::Le,
                        "gt" => FilterOperator::Gt,
                        "ge" => FilterOperator::Ge,
                        "like" => FilterOperator::Like,
                        "ilike" => FilterOperator::ILike,
                        "not_like" => FilterOperator::NotLike,
                        "not_ilike" => FilterOperator::NotILike,
                        "is_null" => FilterOperator::IsNull,
                        "is_not_null" => FilterOperator::IsNotNull,
                        _ => {
                            return Err(BridgeError::rejected(
                                "catalog-browse-filter",
                                "unknown filter operator",
                            ));
                        }
                    };
                    let value = filter.value.as_deref().map(parse_bind_text);
                    Ok(TypedCondition {
                        column: filter.column,
                        operator,
                        value,
                    })
                })
                .collect::<Result<Vec<_>, BridgeError>>()?;
            let dialect = match registered.engine {
                Engine::PostgreSql => BrowseDialect::PostgreSql,
                Engine::ClickHouse => BrowseDialect::ClickHouse,
                Engine::Redis => unreachable!("Redis catalog nodes are not browsable tables"),
            };
            if raw_where
                .as_ref()
                .is_some_and(|fragment| fragment.len() > MAX_BROWSE_VALUE_BYTES)
            {
                return Err(BridgeError::rejected(
                    "catalog-browse-raw-where",
                    "raw WHERE fragment must be at most 65536 bytes",
                ));
            }
            let rendered = BrowsePlan {
                schema: schema.clone(),
                table: table.clone(),
                sort,
                filters,
                raw_where,
                limit: row_count,
                offset: 0,
            }
            .render_sql_for(dialect)
            .map_err(|error| BridgeError::rejected("catalog-browse-plan", error.to_string()))?;
            (
                rendered,
                CopyIdentity {
                    schema,
                    table,
                    identity_columns: Vec::new(),
                    insertable,
                },
            )
        };
        let operation_bytes = self.submit_inner_with_parameters(
            SubmitSpec {
                intent: "browse_object".into(),
                session_id: session_id_bytes,
                statement: Some(rendered.sql),
                result_id: None,
                start_row: None,
                row_count: Some(row_count),
                expected_revision: 0,
            },
            rendered.parameters,
        )?;
        let operation_id = operation_from_bytes(&operation_bytes).map_err(|_| {
            BridgeError::rejected("operation-id", "bridge generated invalid operation id")
        })?;
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        inner
            .operation_copy_identity
            .insert(operation_id, copy_identity);
        Ok(operation_bytes)
    }

    fn relation_structure_inner(
        &self,
        session_id_bytes: Vec<u8>,
        catalog_node_id_bytes: Vec<u8>,
    ) -> Result<BridgeRelationStructure, BridgeError> {
        let session_id = session_from_bytes(&session_id_bytes)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let catalog_node_id = catalog_node_from_bytes(&catalog_node_id_bytes).map_err(|_| {
            BridgeError::rejected("bad-catalog-node-id", "catalog node id must be 16 bytes")
        })?;
        let (driver, namespace, relation) = {
            let guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_ref().ok_or(BridgeError::RuntimeUnavailable)?;
            let registered = inner
                .sessions
                .get(&session_id)
                .ok_or(BridgeError::UnknownSession)?;
            let node = inner
                .catalog_nodes
                .get(&(session_id, catalog_node_id))
                .ok_or_else(|| {
                    BridgeError::rejected(
                        "unknown-catalog-node",
                        "catalog node is stale or unknown",
                    )
                })?;
            let supported = matches!(
                (registered.engine, node.kind()),
                (Engine::PostgreSql, CatalogNodeKind::PostgreSqlObject(_))
                    | (Engine::ClickHouse, CatalogNodeKind::ClickHouseObject(_))
            );
            if !supported {
                return Err(BridgeError::rejected(
                    "relation-structure-kind",
                    "structure snapshot requires a PostgreSQL or ClickHouse object",
                ));
            }
            let parent = node
                .parent_id()
                .and_then(|id| inner.catalog_nodes.get(&(session_id, id)))
                .ok_or_else(|| BridgeError::rejected("catalog-parent", "object parent is stale"))?;
            let driver = inner
                .service
                .session(session_id)
                .ok_or(BridgeError::UnknownSession)?;
            (driver, parent.name().to_owned(), node.name().to_owned())
        };
        let snapshot = self
            .runtime
            .block_on(load_structure_snapshot(driver, namespace, relation))?
            .map_err(|error| BridgeError::rejected("relation-structure", error.to_string()))?;
        Ok(BridgeRelationStructure {
            engine: match snapshot.engine {
                Engine::PostgreSql => "postgresql",
                Engine::ClickHouse => "clickhouse",
                Engine::Redis => "redis",
            }
            .into(),
            namespace: snapshot.namespace,
            relation: snapshot.relation,
            columns: snapshot
                .columns
                .into_iter()
                .map(|column| BridgeRelationColumn {
                    name: column.name,
                    data_type: column.data_type,
                    nullable: column.nullable,
                    default_expression: column.default_expression,
                    comment: column.comment,
                    primary_key: column.primary_key,
                    sorting_key: column.sorting_key,
                })
                .collect(),
            indexes: snapshot
                .indexes
                .into_iter()
                .map(|index| BridgeRelationIndex {
                    kind: index.kind,
                    name: index.name,
                    definition: index.definition,
                })
                .collect(),
            constraints: snapshot
                .constraints
                .into_iter()
                .map(|constraint| BridgeRelationConstraint {
                    kind: constraint.kind,
                    name: constraint.name,
                    definition: constraint.definition,
                })
                .collect(),
            facts: snapshot
                .facts
                .into_iter()
                .map(|fact| BridgeRelationFact {
                    name: fact.name,
                    value: fact.value,
                })
                .collect(),
            ddl: snapshot.ddl,
        })
    }

    fn redis_key_view_inner(
        &self,
        session_id_bytes: Vec<u8>,
        catalog_node_id_bytes: Vec<u8>,
        collection_skip: u64,
    ) -> Result<BridgeRedisKeyView, BridgeError> {
        if collection_skip > 1_000_000 {
            return Err(BridgeError::rejected(
                "redis-key-page",
                "Redis key collection offset exceeds limit",
            ));
        }
        let session_id = session_from_bytes(&session_id_bytes)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let catalog_node_id = catalog_node_from_bytes(&catalog_node_id_bytes).map_err(|_| {
            BridgeError::rejected("bad-catalog-node-id", "catalog node id must be 16 bytes")
        })?;
        let (driver, key) = {
            let guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_ref().ok_or(BridgeError::RuntimeUnavailable)?;
            let registered = inner
                .sessions
                .get(&session_id)
                .ok_or(BridgeError::UnknownSession)?;
            let node = inner
                .catalog_nodes
                .get(&(session_id, catalog_node_id))
                .ok_or_else(|| {
                    BridgeError::rejected(
                        "unknown-catalog-node",
                        "catalog node is stale or unknown",
                    )
                })?;
            if registered.engine != Engine::Redis
                || !matches!(node.kind(), CatalogNodeKind::RedisKey(_))
            {
                return Err(BridgeError::rejected(
                    "redis-key-kind",
                    "Redis key view requires a cached Redis key node",
                ));
            }
            let driver = inner
                .service
                .session(session_id)
                .ok_or(BridgeError::UnknownSession)?;
            (driver, decode_redis_catalog_key(node.name())?)
        };
        let (kind, lines, next_skip) = self
            .runtime
            .block_on(driver.redis_key_view_lines(&key, collection_skip))?
            .map_err(|error| BridgeError::rejected("redis-key-view", error.to_string()))?;
        Ok(BridgeRedisKeyView {
            kind,
            lines,
            next_skip,
        })
    }

    fn redis_overview_inner(
        &self,
        session_id_bytes: Vec<u8>,
    ) -> Result<BridgeRedisOverview, BridgeError> {
        let session_id = session_from_bytes(&session_id_bytes)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let driver = {
            let guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_ref().ok_or(BridgeError::RuntimeUnavailable)?;
            let registered = inner
                .sessions
                .get(&session_id)
                .ok_or(BridgeError::UnknownSession)?;
            if registered.engine != Engine::Redis {
                return Err(BridgeError::rejected(
                    "redis-overview-engine",
                    "Redis overview requires a Redis session",
                ));
            }
            inner
                .service
                .session(session_id)
                .ok_or(BridgeError::UnknownSession)?
        };
        let (sampled_at_ms, lines) = self
            .runtime
            .block_on(driver.redis_info_lines())?
            .map_err(|error| BridgeError::rejected("redis-overview", error.to_string()))?;
        Ok(BridgeRedisOverview {
            sampled_at_ms,
            lines,
        })
    }

    fn start_redis_subscription_inner(
        &self,
        session_id_bytes: Vec<u8>,
        selector: String,
        pattern: bool,
    ) -> Result<Vec<u8>, BridgeError> {
        self.ensure_runtime_inner()?;
        let selector = selector.trim().to_owned();
        if selector.is_empty() {
            return Err(BridgeError::rejected(
                "redis-subscription-selector",
                "channel or pattern must not be empty",
            ));
        }
        let bounded_selector =
            BoundedBytes::copy_from_slice(selector.as_bytes(), ByteLimit::new(256)).map_err(
                |error| BridgeError::rejected("redis-subscription-selector", error.to_string()),
            )?;
        let session_id = session_from_bytes(&session_id_bytes)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let (operation_id, identity, driver) = {
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
            if !inner.accepting {
                return Err(BridgeError::ShuttingDown);
            }
            let registered = inner
                .sessions
                .get(&session_id)
                .ok_or(BridgeError::UnknownSession)?;
            if registered.engine != Engine::Redis {
                return Err(BridgeError::rejected(
                    "redis-subscription-engine",
                    "subscriptions require a Redis session",
                ));
            }
            let operation_id = inner.ids.operation();
            let identity = PageIdentity::new(inner.ids.result(), Revision::INITIAL, Engine::Redis);
            let driver = inner
                .service
                .session(session_id)
                .ok_or(BridgeError::UnknownSession)?;
            (operation_id, identity, driver)
        };
        let (cancel, mut cancellation) = tokio::sync::watch::channel(false);
        {
            let mut tasks = self.redis_subscriptions.lock().map_err(|_| {
                BridgeError::rejected(
                    "redis-subscription-lock",
                    "subscription registry mutex poisoned",
                )
            })?;
            if tasks
                .values()
                .filter(|task| {
                    task.phase == "connecting"
                        || task.phase == "listening"
                        || task.phase == "cancel_requested"
                })
                .count()
                >= 4
            {
                return Err(BridgeError::rejected(
                    "redis-subscription-capacity",
                    "at most four Redis subscriptions may run",
                ));
            }
            if tasks.values().any(|task| {
                task.session_id == session_id
                    && (task.phase == "connecting"
                        || task.phase == "listening"
                        || task.phase == "cancel_requested")
            }) {
                return Err(BridgeError::rejected(
                    "redis-subscription-active",
                    "this Redis session already has an active subscription",
                ));
            }
            while tasks.len() >= 256 {
                let Some(completed) = tasks
                    .iter()
                    .find(|(_, task)| {
                        task.phase != "connecting"
                            && task.phase != "listening"
                            && task.phase != "cancel_requested"
                    })
                    .map(|(operation_id, _)| *operation_id)
                else {
                    return Err(BridgeError::rejected(
                        "redis-subscription-capacity",
                        "Redis subscription status registry is full",
                    ));
                };
                tasks.remove(&completed);
            }
            tasks.insert(
                operation_id,
                RedisSubscriptionTask {
                    session_id,
                    selector: selector.clone(),
                    pattern,
                    phase: "connecting".into(),
                    messages: VecDeque::new(),
                    total_received: 0,
                    discontinuities: 0,
                    summary: "Connecting subscription".into(),
                    cancel,
                },
            );
        }
        let tasks = Arc::clone(&self.redis_subscriptions);
        self.runtime.spawn(async move {
            let options = RedisSubscriptionOptions::new(
                PageLimits::new(16, if pattern { 3 } else { 2 }, 64 * 1024, 8 * 1024),
                1_024,
                64,
            );
            let request = DriverPageRequest::RedisSubscribe {
                selector: bounded_selector,
                kind: if pattern {
                    RedisSubscriptionKind::Pattern
                } else {
                    RedisSubscriptionKind::Channel
                },
                options,
            };
            let started = tokio::select! {
                result = driver.start_page_stream(request) => Some(result),
                _ = cancellation.changed() => None,
            };
            let Some(started) = started else {
                set_redis_subscription_terminal(
                    &tasks,
                    operation_id,
                    "cancelled",
                    "Subscription cancelled",
                );
                return;
            };
            let mut stream = match started {
                Ok(stream) => stream,
                Err(error) => {
                    set_redis_subscription_terminal(
                        &tasks,
                        operation_id,
                        "failed",
                        &format!("Subscription failed: {error}"),
                    );
                    return;
                }
            };
            if let Ok(mut guard) = tasks.lock()
                && let Some(task) = guard.get_mut(&operation_id)
            {
                task.phase = "listening".into();
                task.summary = "Listening for messages".into();
            }
            let mut start_row = 0_u64;
            loop {
                let next = tokio::select! {
                    page = stream.next_page(identity, start_row) => Some(page),
                    _ = cancellation.changed() => None,
                };
                let Some(next) = next else {
                    set_redis_subscription_terminal(
                        &tasks,
                        operation_id,
                        "cancelled",
                        "Subscription cancelled",
                    );
                    break;
                };
                match next {
                    Ok(Some(page)) => {
                        let envelope = page.envelope();
                        if let Ok(mut guard) = tasks.lock()
                            && let Some(task) = guard.get_mut(&operation_id)
                        {
                            if envelope
                                .warnings()
                                .contains(PageWarning::DeliveryDiscontinuity)
                            {
                                task.discontinuities = task.discontinuities.saturating_add(1);
                                task.summary = "Listening; delivery gap observed".into();
                            }
                            for row in 0..envelope.row_count() {
                                let mut fields =
                                    Vec::with_capacity(envelope.column_count() as usize);
                                for column in 0..envelope.column_count() {
                                    match page.cell(row, column) {
                                        Ok(cell) => fields
                                            .push(render_redis_subscription_cell(cell.bytes())),
                                        Err(_) => fields.push("<unavailable>".into()),
                                    }
                                }
                                task.messages.push_back(fields.join(" · "));
                                task.total_received = task.total_received.saturating_add(1);
                                while task.messages.len() > 256 {
                                    task.messages.pop_front();
                                }
                            }
                        }
                        start_row = start_row.saturating_add(u64::from(envelope.row_count()));
                    }
                    Ok(None) => {
                        set_redis_subscription_terminal(
                            &tasks,
                            operation_id,
                            "completed",
                            "Subscription ended",
                        );
                        break;
                    }
                    Err(error) => {
                        set_redis_subscription_terminal(
                            &tasks,
                            operation_id,
                            "failed",
                            &format!("Subscription failed: {error}"),
                        );
                        break;
                    }
                }
            }
        })?;
        Ok(operation_bytes(operation_id))
    }

    fn redis_subscription_status_inner(
        &self,
        operation_id_bytes: Vec<u8>,
    ) -> Result<BridgeRedisSubscriptionStatus, BridgeError> {
        let operation_id = operation_from_bytes(&operation_id_bytes)
            .map_err(|_| BridgeError::rejected("operation-id", "invalid operation id"))?;
        let tasks = self.redis_subscriptions.lock().map_err(|_| {
            BridgeError::rejected(
                "redis-subscription-lock",
                "subscription registry mutex poisoned",
            )
        })?;
        let task = tasks
            .get(&operation_id)
            .ok_or(BridgeError::UnknownOperation)?;
        Ok(BridgeRedisSubscriptionStatus {
            operation_id: operation_id_bytes,
            selector: task.selector.clone(),
            pattern: task.pattern,
            phase: task.phase.clone(),
            messages: task.messages.iter().cloned().collect(),
            total_received: task.total_received,
            discontinuities: task.discontinuities,
            summary: task.summary.clone(),
        })
    }

    fn cancel_redis_subscription_inner(
        &self,
        operation_id_bytes: Vec<u8>,
    ) -> Result<bool, BridgeError> {
        let operation_id = operation_from_bytes(&operation_id_bytes)
            .map_err(|_| BridgeError::rejected("operation-id", "invalid operation id"))?;
        let mut tasks = self.redis_subscriptions.lock().map_err(|_| {
            BridgeError::rejected(
                "redis-subscription-lock",
                "subscription registry mutex poisoned",
            )
        })?;
        let task = tasks
            .get_mut(&operation_id)
            .ok_or(BridgeError::UnknownOperation)?;
        if task.phase != "connecting"
            && task.phase != "listening"
            && task.phase != "cancel_requested"
        {
            return Ok(false);
        }
        if task.phase == "cancel_requested" {
            return Ok(true);
        }
        let _ = task.cancel.send(true);
        task.phase = "cancel_requested".into();
        task.summary = "Cancellation requested".into();
        Ok(true)
    }

    fn postgres_driver(
        &self,
        session_id_bytes: &[u8],
    ) -> Result<std::sync::Arc<dyn DriverSession>, BridgeError> {
        let session_id = session_from_bytes(session_id_bytes)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_ref().ok_or(BridgeError::RuntimeUnavailable)?;
        let registered = inner
            .sessions
            .get(&session_id)
            .ok_or(BridgeError::UnknownSession)?;
        if registered.engine != Engine::PostgreSql {
            return Err(BridgeError::rejected(
                "postgres-activity-engine",
                "PostgreSQL activity requires a PostgreSQL session",
            ));
        }
        inner
            .service
            .session(session_id)
            .ok_or(BridgeError::UnknownSession)
    }

    fn postgres_activity_inner(
        &self,
        session_id_bytes: Vec<u8>,
    ) -> Result<Vec<BridgePostgresActivityRow>, BridgeError> {
        self.ensure_runtime_inner()?;
        let driver = self.postgres_driver(&session_id_bytes)?;
        let rows = self
            .runtime
            .block_on(driver.postgres_activity())?
            .map_err(|error| bridge_activity_error("postgres-activity", error))?;
        Ok(rows
            .into_iter()
            .map(|row| BridgePostgresActivityRow {
                pid: row.pid(),
                user: row.user().to_owned(),
                application: row.application().to_owned(),
                state: row.state().to_owned(),
                query_preview: row.query_preview().to_owned(),
            })
            .collect())
    }

    fn postgres_relationships_inner(
        &self,
        session_id_bytes: Vec<u8>,
        catalog_node_id_bytes: Vec<u8>,
    ) -> Result<BridgeRelationshipSnapshot, BridgeError> {
        self.ensure_runtime_inner()?;
        let session_id = session_from_bytes(&session_id_bytes)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let catalog_node_id = catalog_node_from_bytes(&catalog_node_id_bytes).map_err(|_| {
            BridgeError::rejected("bad-catalog-node-id", "catalog node id must be 16 bytes")
        })?;
        let (driver, namespace, relation) = {
            let guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_ref().ok_or(BridgeError::RuntimeUnavailable)?;
            let registered = inner
                .sessions
                .get(&session_id)
                .ok_or(BridgeError::UnknownSession)?;
            if registered.engine != Engine::PostgreSql {
                return Err(BridgeError::rejected(
                    "postgres-relationships-engine",
                    "relationships require a PostgreSQL session",
                ));
            }
            let node = inner
                .catalog_nodes
                .get(&(session_id, catalog_node_id))
                .ok_or_else(|| {
                    BridgeError::rejected(
                        "unknown-catalog-node",
                        "catalog node is stale or unknown",
                    )
                })?;
            if !matches!(node.kind(), CatalogNodeKind::PostgreSqlObject(_)) {
                return Err(BridgeError::rejected(
                    "postgres-relationships-kind",
                    "relationships require a PostgreSQL relation",
                ));
            }
            let parent = node
                .parent_id()
                .and_then(|id| inner.catalog_nodes.get(&(session_id, id)))
                .ok_or_else(|| BridgeError::rejected("catalog-parent", "object parent is stale"))?;
            let driver = inner
                .service
                .session(session_id)
                .ok_or(BridgeError::UnknownSession)?;
            (driver, parent.name().to_owned(), node.name().to_owned())
        };
        let (graph, truncated) = self
            .runtime
            .block_on(driver.postgres_relationships(&namespace, &relation))?
            .map_err(|error| BridgeError::rejected("postgres-relationships", error.to_string()))?;
        Ok(BridgeRelationshipSnapshot {
            namespace,
            relation,
            edges: graph
                .edges
                .into_iter()
                .map(|edge| BridgeRelationshipEdge {
                    from_schema: edge.from_schema,
                    from_table: edge.from_table,
                    from_column: edge.from_column,
                    to_schema: edge.to_schema,
                    to_table: edge.to_table,
                    to_column: edge.to_column,
                })
                .collect(),
            truncated,
        })
    }

    fn postgres_roles_inner(
        &self,
        session_id_bytes: Vec<u8>,
        catalog_node_id_bytes: Option<Vec<u8>>,
    ) -> Result<BridgeRoleSnapshot, BridgeError> {
        self.ensure_runtime_inner()?;
        let session_id = session_from_bytes(&session_id_bytes)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let (driver, scope) = {
            let guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_ref().ok_or(BridgeError::RuntimeUnavailable)?;
            let registered = inner
                .sessions
                .get(&session_id)
                .ok_or(BridgeError::UnknownSession)?;
            if registered.engine != Engine::PostgreSql {
                return Err(BridgeError::rejected(
                    "postgres-roles-engine",
                    "roles require a PostgreSQL session",
                ));
            }
            let scope = catalog_node_id_bytes
                .map(|bytes| {
                    let node_id = catalog_node_from_bytes(&bytes).map_err(|_| {
                        BridgeError::rejected(
                            "bad-catalog-node-id",
                            "catalog node id must be 16 bytes",
                        )
                    })?;
                    let node =
                        inner
                            .catalog_nodes
                            .get(&(session_id, node_id))
                            .ok_or_else(|| {
                                BridgeError::rejected(
                                    "unknown-catalog-node",
                                    "catalog node is stale or unknown",
                                )
                            })?;
                    if !matches!(node.kind(), CatalogNodeKind::PostgreSqlObject(_)) {
                        return Err(BridgeError::rejected(
                            "postgres-roles-kind",
                            "privilege scope requires a PostgreSQL relation",
                        ));
                    }
                    let parent = node
                        .parent_id()
                        .and_then(|id| inner.catalog_nodes.get(&(session_id, id)))
                        .ok_or_else(|| {
                            BridgeError::rejected("catalog-parent", "object parent is stale")
                        })?;
                    Ok((parent.name().to_owned(), node.name().to_owned()))
                })
                .transpose()?;
            let driver = inner
                .service
                .session(session_id)
                .ok_or(BridgeError::UnknownSession)?;
            (driver, scope)
        };
        let snapshot = self
            .runtime
            .block_on(driver.postgres_roles(
                scope.as_ref().map(|value| value.0.as_str()),
                scope.as_ref().map(|value| value.1.as_str()),
            ))?
            .map_err(|error| BridgeError::rejected("postgres-roles", error.to_string()))?;
        Ok(BridgeRoleSnapshot {
            current_user: snapshot.current_user,
            roles: snapshot.roles,
            memberships: snapshot
                .memberships
                .into_iter()
                .map(|edge| BridgeRoleMembership {
                    role: edge.role,
                    member: edge.member,
                    inherit_option: edge.inherit_option,
                    admin_option: edge.admin_option,
                    set_option: edge.set_option,
                })
                .collect(),
            effective_roles: snapshot.effective_roles,
            cycle_edges: snapshot
                .cycle_edges
                .into_iter()
                .map(|(from, to)| format!("{from} -> {to}"))
                .collect(),
            privileges: snapshot
                .privileges
                .into_iter()
                .map(|row| BridgeRolePrivilege {
                    grantee: row.grantee,
                    privilege: row.privilege,
                    object: row.object,
                    grantable: row.is_grantable,
                })
                .collect(),
            privilege_scope: scope.map(|(schema, relation)| format!("{schema}.{relation}")),
            privileges_unavailable: snapshot.privileges_unavailable,
            truncated: snapshot.truncated,
        })
    }

    fn stage_postgres_role_change_inner(
        &self,
        request: BridgeRoleChangeRequest,
    ) -> Result<BridgeRoleChangeReview, BridgeError> {
        self.ensure_runtime_inner()?;
        let session_id = session_from_bytes(&request.session_id)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let (driver, registered, relation_scope) = {
            let guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_ref().ok_or(BridgeError::RuntimeUnavailable)?;
            let registered = inner
                .sessions
                .get(&session_id)
                .ok_or(BridgeError::UnknownSession)?
                .clone();
            if registered.engine != Engine::PostgreSql {
                return Err(BridgeError::rejected(
                    "postgres-role-change-engine",
                    "role changes require PostgreSQL",
                ));
            }
            let relation_scope = request
                .catalog_node_id
                .as_ref()
                .map(|bytes| {
                    let node_id = catalog_node_from_bytes(bytes).map_err(|_| {
                        BridgeError::rejected(
                            "bad-catalog-node-id",
                            "catalog node id must be 16 bytes",
                        )
                    })?;
                    let node =
                        inner
                            .catalog_nodes
                            .get(&(session_id, node_id))
                            .ok_or_else(|| {
                                BridgeError::rejected(
                                    "unknown-catalog-node",
                                    "catalog node is stale or unknown",
                                )
                            })?;
                    if !matches!(node.kind(), CatalogNodeKind::PostgreSqlObject(_)) {
                        return Err(BridgeError::rejected(
                            "postgres-role-change-kind",
                            "privilege changes require a PostgreSQL relation",
                        ));
                    }
                    let parent = node
                        .parent_id()
                        .and_then(|id| inner.catalog_nodes.get(&(session_id, id)))
                        .ok_or_else(|| {
                            BridgeError::rejected("catalog-parent", "object parent is stale")
                        })?;
                    Ok((parent.name().to_owned(), node.name().to_owned()))
                })
                .transpose()?;
            let driver = inner
                .service
                .session(session_id)
                .ok_or(BridgeError::UnknownSession)?;
            (driver, registered, relation_scope)
        };
        let current_user = self
            .runtime
            .block_on(driver.postgres_roles(None, None))?
            .map_err(|error| BridgeError::rejected("postgres-role-change", error.to_string()))?
            .current_user;
        let kind = match request.kind.as_str() {
            "grant_membership" => RoleChangeKind::GrantMembership {
                role: request.role,
                member: request.member_or_grantee,
            },
            "revoke_membership" => RoleChangeKind::RevokeMembership {
                role: request.role,
                member: request.member_or_grantee,
            },
            "grant_privilege" | "revoke_privilege" => {
                let (schema, table) = relation_scope.ok_or_else(|| {
                    BridgeError::rejected(
                        "postgres-role-change-scope",
                        "privilege changes require a selected relation",
                    )
                })?;
                if request.kind == "grant_privilege" {
                    RoleChangeKind::GrantTablePrivilege {
                        schema,
                        table,
                        grantee: request.member_or_grantee,
                        privilege: request.privilege,
                    }
                } else {
                    RoleChangeKind::RevokeTablePrivilege {
                        schema,
                        table,
                        grantee: request.member_or_grantee,
                        privilege: request.privilege,
                    }
                }
            }
            _ => {
                return Err(BridgeError::rejected(
                    "postgres-role-change-kind",
                    "unknown role change kind",
                ));
            }
        };
        let scope = OperationScope::new(
            registered.profile_id,
            registered.session_id,
            registered.context_id,
        );
        let summary = role_change_summary(&kind);
        let plan = RoleChangePlan::new(
            Engine::PostgreSql,
            scope,
            registered.context_revision,
            &current_user,
            kind,
        )
        .map_err(|error| {
            BridgeError::rejected("postgres-role-change-plan", format!("{error:?}"))
        })?;
        let expires_at_ms = request.now_ms.saturating_add(60_000);
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        let current = inner
            .sessions
            .get(&session_id)
            .ok_or(BridgeError::UnknownSession)?;
        if current.context_revision != registered.context_revision {
            return Err(BridgeError::rejected(
                "postgres-role-change-stale",
                "session context changed during review staging",
            ));
        }
        inner
            .role_reviews
            .retain(|_, entry| request.now_ms < entry.expires_at_ms);
        if inner.role_reviews.len() >= 256 {
            return Err(BridgeError::rejected(
                "postgres-role-change-capacity",
                "too many active role reviews",
            ));
        }
        let token_id = inner.ids.review_token();
        let reviewed = plan
            .review(token_id, request.now_ms, expires_at_ms)
            .map_err(|error| {
                BridgeError::rejected("postgres-role-change-review", format!("{error:?}"))
            })?;
        inner.role_reviews.insert(
            token_id,
            RoleReviewEntry {
                session_id,
                reviewed,
                expires_at_ms,
            },
        );
        Ok(BridgeRoleChangeReview {
            token_id: review_token_bytes(token_id),
            summary,
            expires_at_ms,
        })
    }

    fn apply_postgres_role_change_inner(
        &self,
        token_id_bytes: Vec<u8>,
        session_id_bytes: Vec<u8>,
        now_ms: u64,
        confirmed: bool,
    ) -> Result<String, BridgeError> {
        if !confirmed {
            return Err(BridgeError::rejected(
                "postgres-role-change-confirmation",
                "explicit confirmation is required",
            ));
        }
        let token_id = review_token_from_bytes(&token_id_bytes).map_err(|_| {
            BridgeError::rejected("bad-review-token-id", "review token id must be 16 bytes")
        })?;
        let session_id = session_from_bytes(&session_id_bytes)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let (authorized, driver) = {
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
            let entry = inner.role_reviews.remove(&token_id).ok_or_else(|| {
                BridgeError::rejected(
                    "postgres-role-change-token",
                    "role review token is missing or consumed",
                )
            })?;
            if entry.session_id != session_id {
                return Err(BridgeError::rejected(
                    "postgres-role-change-session",
                    "role review belongs to another session",
                ));
            }
            let registered = inner
                .sessions
                .get(&session_id)
                .ok_or(BridgeError::UnknownSession)?;
            let scope = OperationScope::new(
                registered.profile_id,
                registered.session_id,
                registered.context_id,
            );
            let authorized = entry
                .reviewed
                .authorize(now_ms, scope, registered.context_revision)
                .map_err(|error| {
                    BridgeError::rejected("postgres-role-change-authorize", format!("{error:?}"))
                })?;
            let driver = inner
                .service
                .session(session_id)
                .ok_or(BridgeError::UnknownSession)?;
            (authorized, driver)
        };
        self.runtime
            .block_on(driver.apply_postgres_role_change(authorized))?
            .map_err(|error| {
                BridgeError::rejected("postgres-role-change-apply", error.to_string())
            })?;
        Ok("Role change applied".into())
    }

    fn revoke_postgres_role_change_inner(
        &self,
        token_id_bytes: Vec<u8>,
    ) -> Result<bool, BridgeError> {
        let token_id = review_token_from_bytes(&token_id_bytes).map_err(|_| {
            BridgeError::rejected("bad-review-token-id", "review token id must be 16 bytes")
        })?;
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        Ok(inner.role_reviews.remove(&token_id).is_some())
    }

    fn stage_ddl_change_inner(
        &self,
        request: BridgeDdlChangeRequest,
    ) -> Result<BridgeDdlChangeReview, BridgeError> {
        self.ensure_runtime_inner()?;
        let session_id = session_from_bytes(&request.session_id)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let node_id = catalog_node_from_bytes(&request.catalog_node_id).map_err(|_| {
            BridgeError::rejected("bad-catalog-node-id", "catalog node id must be 16 bytes")
        })?;
        let kind = match request.kind.as_str() {
            "add_column" => DdlKind::AddColumn,
            "drop_column" => DdlKind::DropColumn,
            "create_index" => DdlKind::CreateIndex,
            "drop_index" => DdlKind::DropIndex,
            "add_constraint" => DdlKind::AddConstraint,
            "drop_constraint" => DdlKind::DropConstraint,
            _ => {
                return Err(BridgeError::rejected(
                    "ddl-change-kind",
                    "unknown structure change kind",
                ));
            }
        };
        let destructive = matches!(
            kind,
            DdlKind::DropColumn | DdlKind::DropIndex | DdlKind::DropConstraint
        );
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        if !inner.accepting {
            return Err(BridgeError::ShuttingDown);
        }
        let registered = inner
            .sessions
            .get(&session_id)
            .ok_or(BridgeError::UnknownSession)?
            .clone();
        if registered.engine != Engine::PostgreSql {
            return Err(BridgeError::rejected(
                "ddl-change-engine",
                "structure changes require PostgreSQL",
            ));
        }
        let node = inner
            .catalog_nodes
            .get(&(session_id, node_id))
            .ok_or_else(|| {
                BridgeError::rejected("unknown-catalog-node", "catalog node is stale or unknown")
            })?;
        if !matches!(
            node.kind(),
            CatalogNodeKind::PostgreSqlObject(
                PostgreSqlObjectKind::Table | PostgreSqlObjectKind::PartitionedTable
            )
        ) {
            return Err(BridgeError::rejected(
                "ddl-change-target",
                "structure changes require a PostgreSQL table",
            ));
        }
        let parent = node
            .parent_id()
            .and_then(|id| inner.catalog_nodes.get(&(session_id, id)))
            .ok_or_else(|| BridgeError::rejected("catalog-parent", "object parent is stale"))?;
        let target = DdlTarget::PostgreSqlRelation {
            schema: parent.name().to_owned(),
            relation: node.name().to_owned(),
        };
        let object_name =
            (!request.object_name.trim().is_empty()).then(|| request.object_name.trim().to_owned());
        let definition =
            (!request.definition.trim().is_empty()).then(|| request.definition.trim().to_owned());
        let scope = OperationScope::new(
            registered.profile_id,
            registered.session_id,
            registered.context_id,
        );
        let plan = DdlPlan::new(
            kind,
            Engine::PostgreSql,
            scope,
            registered.context_revision,
            target,
            object_name,
            definition,
        )
        .map_err(|error| BridgeError::rejected("ddl-change-plan", error.to_string()))?;
        let preview = preview_postgres_ddl(&plan)?;
        inner
            .ddl_reviews
            .retain(|_, entry| request.now_ms < entry.expires_at_ms);
        if inner.ddl_reviews.len() >= 256 {
            return Err(BridgeError::rejected(
                "ddl-change-capacity",
                "too many active structure reviews",
            ));
        }
        let token_id = inner.ids.review_token();
        let expires_at_ms = request.now_ms.saturating_add(60_000);
        inner.ddl_reviews.insert(
            token_id,
            DdlReviewEntry {
                session_id,
                plan,
                expires_at_ms,
                confirmation: None,
            },
        );
        Ok(BridgeDdlChangeReview {
            token_id: review_token_bytes(token_id),
            preview,
            destructive,
            rollback_summary: "PostgreSQL applies this statement atomically; TableRock does not automatically roll it back after observed success.".into(),
            expires_at_ms,
        })
    }

    fn apply_ddl_change_inner(
        &self,
        token_id_bytes: Vec<u8>,
        session_id_bytes: Vec<u8>,
        now_ms: u64,
        confirmed: bool,
    ) -> Result<String, BridgeError> {
        if !confirmed {
            return Err(BridgeError::rejected(
                "ddl-change-confirmation",
                "explicit confirmation is required",
            ));
        }
        let token_id = review_token_from_bytes(&token_id_bytes).map_err(|_| {
            BridgeError::rejected("bad-review-token-id", "review token id must be 16 bytes")
        })?;
        let session_id = session_from_bytes(&session_id_bytes)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let (plan, driver) = {
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
            if inner
                .ddl_reviews
                .get(&token_id)
                .is_some_and(|entry| entry.confirmation.is_some())
            {
                return Err(BridgeError::rejected(
                    "ddl-change-token-kind",
                    "table-operation token requires its specific apply path",
                ));
            }
            let entry = inner.ddl_reviews.remove(&token_id).ok_or_else(|| {
                BridgeError::rejected(
                    "ddl-change-token",
                    "structure review token is missing or consumed",
                )
            })?;
            if entry.session_id != session_id {
                return Err(BridgeError::rejected(
                    "ddl-change-session",
                    "structure review belongs to another session",
                ));
            }
            if now_ms >= entry.expires_at_ms {
                return Err(BridgeError::rejected(
                    "ddl-change-expired",
                    "structure review expired",
                ));
            }
            let registered = inner
                .sessions
                .get(&session_id)
                .ok_or(BridgeError::UnknownSession)?;
            let current_scope = OperationScope::new(
                registered.profile_id,
                registered.session_id,
                registered.context_id,
            );
            if entry.plan.scope != current_scope
                || entry.plan.revision != registered.context_revision
            {
                return Err(BridgeError::rejected(
                    "ddl-change-stale",
                    "session context changed after review",
                ));
            }
            let driver = inner
                .service
                .session(session_id)
                .ok_or(BridgeError::UnknownSession)?;
            (entry.plan, driver)
        };
        self.runtime
            .block_on(driver.execute_ddl_plan(plan))?
            .map_err(|error| BridgeError::rejected("ddl-change-apply", error.to_string()))?;
        Ok("Structure change applied".into())
    }

    fn revoke_ddl_change_inner(&self, token_id_bytes: Vec<u8>) -> Result<bool, BridgeError> {
        let token_id = review_token_from_bytes(&token_id_bytes).map_err(|_| {
            BridgeError::rejected("bad-review-token-id", "review token id must be 16 bytes")
        })?;
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        Ok(inner.ddl_reviews.remove(&token_id).is_some())
    }

    fn stage_table_operation_inner(
        &self,
        request: BridgeTableOperationRequest,
    ) -> Result<BridgeTableOperationReview, BridgeError> {
        self.ensure_runtime_inner()?;
        let session_id = session_from_bytes(&request.session_id)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let node_id = catalog_node_from_bytes(&request.catalog_node_id).map_err(|_| {
            BridgeError::rejected("bad-catalog-node-id", "catalog node id must be 16 bytes")
        })?;
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        if !inner.accepting {
            return Err(BridgeError::ShuttingDown);
        }
        let registered = inner
            .sessions
            .get(&session_id)
            .ok_or(BridgeError::UnknownSession)?
            .clone();
        let node = inner
            .catalog_nodes
            .get(&(session_id, node_id))
            .ok_or_else(|| {
                BridgeError::rejected("unknown-catalog-node", "catalog node is stale or unknown")
            })?;
        let parent = node
            .parent_id()
            .and_then(|id| inner.catalog_nodes.get(&(session_id, id)))
            .ok_or_else(|| BridgeError::rejected("catalog-parent", "object parent is stale"))?;
        let (kind, target) = match (registered.engine, request.kind.as_str(), node.kind()) {
            (
                Engine::PostgreSql,
                "rename",
                CatalogNodeKind::PostgreSqlObject(
                    PostgreSqlObjectKind::Table | PostgreSqlObjectKind::PartitionedTable,
                ),
            ) => (
                DdlKind::RenameTable,
                DdlTarget::PostgreSqlRelation {
                    schema: parent.name().to_owned(),
                    relation: node.name().to_owned(),
                },
            ),
            (
                Engine::PostgreSql,
                "truncate",
                CatalogNodeKind::PostgreSqlObject(
                    PostgreSqlObjectKind::Table | PostgreSqlObjectKind::PartitionedTable,
                ),
            ) => (
                DdlKind::TruncateTable,
                DdlTarget::PostgreSqlRelation {
                    schema: parent.name().to_owned(),
                    relation: node.name().to_owned(),
                },
            ),
            (
                Engine::PostgreSql,
                "drop",
                CatalogNodeKind::PostgreSqlObject(
                    PostgreSqlObjectKind::Table | PostgreSqlObjectKind::PartitionedTable,
                ),
            ) => (
                DdlKind::DropTable,
                DdlTarget::PostgreSqlRelation {
                    schema: parent.name().to_owned(),
                    relation: node.name().to_owned(),
                },
            ),
            (
                Engine::PostgreSql,
                "vacuum",
                CatalogNodeKind::PostgreSqlObject(
                    PostgreSqlObjectKind::Table | PostgreSqlObjectKind::PartitionedTable,
                ),
            ) => (
                DdlKind::Vacuum,
                DdlTarget::PostgreSqlRelation {
                    schema: parent.name().to_owned(),
                    relation: node.name().to_owned(),
                },
            ),
            (
                Engine::PostgreSql,
                "analyze",
                CatalogNodeKind::PostgreSqlObject(
                    PostgreSqlObjectKind::Table | PostgreSqlObjectKind::PartitionedTable,
                ),
            ) => (
                DdlKind::Analyze,
                DdlTarget::PostgreSqlRelation {
                    schema: parent.name().to_owned(),
                    relation: node.name().to_owned(),
                },
            ),
            (
                Engine::ClickHouse,
                "optimize",
                CatalogNodeKind::ClickHouseObject(ClickHouseObjectKind::Table),
            ) => (
                DdlKind::Optimize,
                DdlTarget::ClickHouseTable {
                    database: parent.name().to_owned(),
                    table: node.name().to_owned(),
                },
            ),
            _ => {
                return Err(BridgeError::rejected(
                    "table-operation-capability",
                    "operation is unavailable for this engine or target",
                ));
            }
        };
        let new_name = (kind == DdlKind::RenameTable)
            .then(|| request.new_name.trim().to_owned())
            .filter(|name| !name.is_empty());
        let scope = OperationScope::new(
            registered.profile_id,
            registered.session_id,
            registered.context_id,
        );
        let plan = DdlPlan::new(
            kind,
            registered.engine,
            scope,
            registered.context_revision,
            target,
            new_name,
            None,
        )
        .map_err(|error| BridgeError::rejected("table-operation-plan", error.to_string()))?;
        let preview = preview_table_operation(&plan)?;
        let target = format!("{}.{}", parent.name(), node.name());
        let confirmation = node.name().to_owned();
        let destructive = matches!(kind, DdlKind::TruncateTable | DdlKind::DropTable);
        inner
            .ddl_reviews
            .retain(|_, entry| request.now_ms < entry.expires_at_ms);
        if inner.ddl_reviews.len() >= 256 {
            return Err(BridgeError::rejected(
                "table-operation-capacity",
                "too many active table-operation reviews",
            ));
        }
        let token_id = inner.ids.review_token();
        let expires_at_ms = request.now_ms.saturating_add(60_000);
        inner.ddl_reviews.insert(
            token_id,
            DdlReviewEntry {
                session_id,
                plan,
                expires_at_ms,
                confirmation: Some(confirmation.clone()),
            },
        );
        Ok(BridgeTableOperationReview {
            token_id: review_token_bytes(token_id),
            target,
            preview,
            destructive,
            confirmation,
            expires_at_ms,
        })
    }

    fn apply_table_operation_inner(
        &self,
        token_id_bytes: Vec<u8>,
        session_id_bytes: Vec<u8>,
        now_ms: u64,
        confirmation: String,
    ) -> Result<String, BridgeError> {
        let token_id = review_token_from_bytes(&token_id_bytes).map_err(|_| {
            BridgeError::rejected("bad-review-token-id", "review token id must be 16 bytes")
        })?;
        let session_id = session_from_bytes(&session_id_bytes)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
        let (plan, driver) = {
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
            let expected = inner
                .ddl_reviews
                .get(&token_id)
                .and_then(|entry| entry.confirmation.as_deref())
                .ok_or_else(|| {
                    BridgeError::rejected(
                        "table-operation-token",
                        "table-operation token is missing, consumed, or wrong kind",
                    )
                })?;
            if confirmation != expected {
                return Err(BridgeError::rejected(
                    "table-operation-confirmation",
                    "confirmation must exactly match the target table name",
                ));
            }
            let entry = inner.ddl_reviews.remove(&token_id).ok_or_else(|| {
                BridgeError::rejected("table-operation-token", "table-operation token is missing")
            })?;
            if entry.session_id != session_id {
                return Err(BridgeError::rejected(
                    "table-operation-session",
                    "table-operation review belongs to another session",
                ));
            }
            if now_ms >= entry.expires_at_ms {
                return Err(BridgeError::rejected(
                    "table-operation-expired",
                    "table-operation review expired",
                ));
            }
            let registered = inner
                .sessions
                .get(&session_id)
                .ok_or(BridgeError::UnknownSession)?;
            let current_scope = OperationScope::new(
                registered.profile_id,
                registered.session_id,
                registered.context_id,
            );
            if entry.plan.scope != current_scope
                || entry.plan.revision != registered.context_revision
            {
                return Err(BridgeError::rejected(
                    "table-operation-stale",
                    "session context changed after review",
                ));
            }
            let driver = inner
                .service
                .session(session_id)
                .ok_or(BridgeError::UnknownSession)?;
            (entry.plan, driver)
        };
        self.runtime
            .block_on(driver.execute_ddl_plan(plan))?
            .map_err(|error| BridgeError::rejected("table-operation-apply", error.to_string()))?;
        Ok("Table operation applied".into())
    }

    fn signal_postgres_backend_inner(
        &self,
        session_id_bytes: Vec<u8>,
        kind: String,
        pid: i32,
    ) -> Result<BridgeBackendSignalOutcome, BridgeError> {
        self.ensure_runtime_inner()?;
        if pid <= 0 {
            return Err(BridgeError::rejected(
                "postgres-activity-pid",
                "backend pid must be positive",
            ));
        }
        let terminate = match kind.as_str() {
            "cancel" => false,
            "terminate" => true,
            _ => {
                return Err(BridgeError::rejected(
                    "postgres-activity-signal",
                    "signal kind must be cancel or terminate",
                ));
            }
        };
        let driver = self.postgres_driver(&session_id_bytes)?;
        let acknowledged = self
            .runtime
            .block_on(driver.signal_postgres_backend(terminate, pid))?
            .map_err(|error| bridge_activity_error("postgres-activity-signal", error))?;
        Ok(BridgeBackendSignalOutcome {
            kind,
            pid,
            acknowledged,
        })
    }

    fn start_postgres_tool_inner(
        &self,
        request: BridgePostgresToolRequest,
    ) -> Result<Vec<u8>, BridgeError> {
        self.ensure_runtime_inner()?;
        let BridgePostgresToolRequest {
            session_id: session_id_bytes,
            kind,
            tool_path,
            file_path,
            content,
            clean,
            no_owner,
        } = request;
        let (tool_name, restore) = postgres_tool_kind(&kind)?;
        if !matches!(content.as_str(), "all" | "schema_only" | "data_only") {
            return Err(BridgeError::rejected(
                "postgres-tool-content",
                "content must be all, schema_only, or data_only",
            ));
        }
        if clean && !restore {
            return Err(BridgeError::rejected(
                "postgres-tool-clean",
                "clean is valid only for restore",
            ));
        }
        let tool = match discover_tool(tool_name, Some(tool_path.as_str())) {
            ToolStatus::Found { path, .. } => path,
            ToolStatus::Missing { .. } => {
                return Err(BridgeError::rejected(
                    "postgres-tool-missing",
                    "selected PostgreSQL tool is unavailable",
                ));
            }
            ToolStatus::VersionProbeFailed { .. } => {
                return Err(BridgeError::rejected(
                    "postgres-tool-version",
                    "selected PostgreSQL tool version probe failed",
                ));
            }
        };
        let file = validate_dump_path(Path::new(&file_path))
            .map_err(|message| BridgeError::rejected("postgres-tool-file", message))?;
        if !file.is_absolute() {
            return Err(BridgeError::rejected(
                "postgres-tool-file",
                "backup path must be absolute",
            ));
        }
        if restore && !file.is_file() {
            return Err(BridgeError::rejected(
                "postgres-tool-file",
                "restore archive must be a regular file",
            ));
        }
        if !restore && !file.parent().is_some_and(|parent| parent.is_dir()) {
            return Err(BridgeError::rejected(
                "postgres-tool-file",
                "backup destination parent must exist",
            ));
        }
        let session_id = session_from_bytes(&session_id_bytes)
            .map_err(|_| BridgeError::rejected("session-id", "invalid session id"))?;
        let (operation_id, connection) = {
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
            let connection = inner
                .sessions
                .get(&session_id)
                .ok_or(BridgeError::UnknownSession)?
                .postgres_tool_connection
                .clone()
                .ok_or_else(|| {
                    BridgeError::rejected(
                        "postgres-tool-session",
                        "backup and restore require a live PostgreSQL connection",
                    )
                })?;
            (inner.ids.operation(), connection)
        };
        let (cancel, receiver) = cancel_channel();
        {
            let mut tasks = self.postgres_tools.lock().map_err(|_| {
                BridgeError::rejected("postgres-tool-lock", "tool registry mutex poisoned")
            })?;
            if tasks
                .values()
                .filter(|task| task.phase == "running" || task.phase == "cancel_requested")
                .count()
                >= 4
            {
                return Err(BridgeError::rejected(
                    "postgres-tool-capacity",
                    "at most four PostgreSQL tool operations may run",
                ));
            }
            while tasks.len() >= 256 {
                let Some(completed) = tasks
                    .iter()
                    .find(|(_, task)| task.phase != "running" && task.phase != "cancel_requested")
                    .map(|(operation_id, _)| *operation_id)
                else {
                    return Err(BridgeError::rejected(
                        "postgres-tool-capacity",
                        "PostgreSQL tool status registry is full",
                    ));
                };
                tasks.remove(&completed);
            }
            tasks.insert(
                operation_id,
                PostgresToolTask {
                    session_id,
                    kind: kind.clone(),
                    phase: "running".into(),
                    summary: "Process started".into(),
                    cancel,
                },
            );
        }
        let tasks = Arc::clone(&self.postgres_tools);
        self.runtime.spawn(async move {
            let password =
                (!connection.password.is_empty()).then_some(connection.password.as_str());
            let outcome = if restore {
                run_pg_restore_configured(
                    &tool,
                    &connection.host,
                    connection.port,
                    &connection.database,
                    &connection.user,
                    password,
                    &file,
                    &content,
                    clean,
                    no_owner,
                    receiver,
                )
                .await
            } else {
                run_pg_dump_configured(
                    &tool,
                    &connection.host,
                    connection.port,
                    &connection.database,
                    &connection.user,
                    password,
                    &file,
                    &content,
                    no_owner,
                    receiver,
                )
                .await
            };
            if let Ok(mut tasks) = tasks.lock()
                && let Some(task) = tasks.get_mut(&operation_id)
            {
                let (phase, summary) = match outcome {
                    PgToolRunOutcome::Succeeded { exit_code } => (
                        "succeeded",
                        format!("Process completed with exit {exit_code}"),
                    ),
                    PgToolRunOutcome::Failed { exit_code, .. } => {
                        ("failed", format!("Process failed with exit {exit_code:?}"))
                    }
                    PgToolRunOutcome::Cancelled => ("cancelled", "Process cancelled".into()),
                    PgToolRunOutcome::SpawnFailed { .. } => {
                        ("failed", "Process could not start".into())
                    }
                };
                task.phase = phase.into();
                task.summary = summary;
            }
        })?;
        Ok(operation_bytes(operation_id))
    }

    fn postgres_tool_status_inner(
        &self,
        operation_id_bytes: Vec<u8>,
    ) -> Result<BridgePostgresToolStatus, BridgeError> {
        let operation_id = operation_from_bytes(&operation_id_bytes)
            .map_err(|_| BridgeError::rejected("operation-id", "invalid operation id"))?;
        let tasks = self.postgres_tools.lock().map_err(|_| {
            BridgeError::rejected("postgres-tool-lock", "tool registry mutex poisoned")
        })?;
        let task = tasks
            .get(&operation_id)
            .ok_or(BridgeError::UnknownOperation)?;
        Ok(BridgePostgresToolStatus {
            operation_id: operation_id_bytes,
            kind: task.kind.clone(),
            phase: task.phase.clone(),
            summary: task.summary.clone(),
        })
    }

    fn cancel_postgres_tool_inner(&self, operation_id_bytes: Vec<u8>) -> Result<bool, BridgeError> {
        let operation_id = operation_from_bytes(&operation_id_bytes)
            .map_err(|_| BridgeError::rejected("operation-id", "invalid operation id"))?;
        let mut tasks = self.postgres_tools.lock().map_err(|_| {
            BridgeError::rejected("postgres-tool-lock", "tool registry mutex poisoned")
        })?;
        let task = tasks
            .get_mut(&operation_id)
            .ok_or(BridgeError::UnknownOperation)?;
        if task.phase != "running" && task.phase != "cancel_requested" {
            return Ok(false);
        }
        if task.cancel.send(true).is_err() {
            return Ok(false);
        }
        task.phase = "cancel_requested".into();
        task.summary = "Cancellation requested".into();
        Ok(true)
    }

    fn format_result_copy_inner(
        &self,
        result_id_bytes: Vec<u8>,
        revision: u64,
        scope: String,
        row: Option<u64>,
        column: Option<u32>,
        format: String,
    ) -> Result<String, BridgeError> {
        let result_id = result_from_bytes(&result_id_bytes)
            .map_err(|_| BridgeError::rejected("bad-result-id", "result id must be 16 bytes"))?;
        let revision = Revision::from_wire_u64(revision);
        let format = match format.as_str() {
            "csv" => CopyFormat::Csv,
            "tsv" => CopyFormat::Tsv,
            "json" => CopyFormat::Json,
            "markdown" => CopyFormat::Markdown,
            "sql_insert" => CopyFormat::SqlInsert,
            "sql_update" => CopyFormat::SqlUpdate,
            _ => {
                return Err(BridgeError::rejected(
                    "copy-format",
                    "unsupported copy format",
                ));
            }
        };
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        let pages = inner
            .results
            .resident_pages(result_id, revision)
            .ok_or(BridgeError::UnknownPage)?;
        let first = pages.first().ok_or(BridgeError::UnknownPage)?;
        let columns = first
            .columns()
            .iter()
            .map(|column| column.name().to_owned())
            .collect::<Vec<_>>();
        let selected_columns: Vec<u32> = match scope.as_str() {
            "cell" => vec![column.ok_or_else(|| {
                BridgeError::rejected("copy-column", "cell scope requires column")
            })?],
            "row" | "loaded" => (0..u32::try_from(columns.len()).unwrap_or(u32::MAX)).collect(),
            _ => {
                return Err(BridgeError::rejected(
                    "copy-scope",
                    "unsupported copy scope",
                ));
            }
        };
        if selected_columns
            .iter()
            .any(|column| usize::try_from(*column).map_or(true, |index| index >= columns.len()))
        {
            return Err(BridgeError::rejected(
                "copy-column",
                "column is outside result",
            ));
        }
        let selected_names = selected_columns
            .iter()
            .map(|column| columns[*column as usize].clone())
            .collect();
        let mut rows = Vec::new();
        for page in &pages {
            let envelope = page.envelope();
            for local_row in 0..envelope.row_count() {
                let absolute_row = envelope.start_row().saturating_add(u64::from(local_row));
                if scope != "loaded" && Some(absolute_row) != row {
                    continue;
                }
                let cells = selected_columns
                    .iter()
                    .map(|column| {
                        page.cell(local_row, *column)
                            .map(copy_cell_from_page)
                            .map_err(|error| BridgeError::rejected("copy-cell", error.to_string()))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                rows.push(cells);
            }
        }
        if scope != "loaded" && rows.len() != 1 {
            return Err(BridgeError::rejected(
                "copy-row",
                "row is outside resident result",
            ));
        }
        let identity = inner.copy_identities.get(&result_id);
        let table_identity = identity.filter(|value| value.insertable);
        format_copy_table(
            &CopyTable {
                columns: selected_names,
                rows,
                base_schema: table_identity.map(|value| value.schema.clone()),
                base_table: table_identity.map(|value| value.table.clone()),
                identity_columns: identity
                    .map(|value| value.identity_columns.clone())
                    .unwrap_or_default(),
            },
            format,
        )
        .map_err(|error| BridgeError::rejected("copy-format", error.to_string()))
    }

    fn open_inner(&self, params: OpenParams) -> Result<Vec<u8>, BridgeError> {
        self.open_inner_for_profile(params, None)
    }

    fn open_inner_for_profile(
        &self,
        params: OpenParams,
        saved_profile_id: Option<ProfileId>,
    ) -> Result<Vec<u8>, BridgeError> {
        self.ensure_runtime_inner()?;
        let engine = parse_engine(&params.engine)?;
        let text = |value: &str, field: &str| {
            BoundedText::copy_from_str(value, ByteLimit::new(256))
                .map_err(|error| BridgeError::rejected(field, error.to_string()))
        };
        let host = text(&params.host, "host")?;
        let database = text(
            if params.database.is_empty() {
                match engine {
                    Engine::PostgreSql => "postgres",
                    Engine::ClickHouse => "default",
                    Engine::Redis => "0",
                }
            } else {
                &params.database
            },
            "database",
        )?;
        let user = text(
            if params.user.is_empty() {
                match engine {
                    Engine::PostgreSql => "postgres",
                    Engine::ClickHouse => "default",
                    Engine::Redis => "",
                }
            } else {
                &params.user
            },
            "user",
        )?;
        let port = params.port;
        let password = params.password.clone();
        let password_opt = if password.is_empty() {
            None
        } else {
            Some(password.as_str())
        };
        let postgres_tool_connection =
            (engine == Engine::PostgreSql).then(|| PostgresToolConnection {
                host: host.as_str().to_owned(),
                port,
                database: database.as_str().to_owned(),
                user: user.as_str().to_owned(),
                password: Arc::new(Zeroizing::new(password.clone())),
            });
        let tls_required = match params.tls_mode.as_str() {
            "" | "off" => false,
            "verify_ca" | "verify_full" => true,
            _ => return Err(BridgeError::rejected("tls-mode", "unknown TLS mode")),
        };

        let session: Box<dyn DriverSession> = self.runtime.block_on(async {
            match engine {
                Engine::PostgreSql => {
                    let session = PostgresSession::connect_with_password(
                        &PostgresConnectConfig::new(
                            host,
                            port,
                            database.clone(),
                            user,
                            if tls_required {
                                PostgresTlsMode::Required
                            } else {
                                PostgresTlsMode::Disabled
                            },
                        ),
                        password_opt,
                    )
                    .await
                    .map_err(|error| BridgeError::rejected("connect", error.to_string()))?;
                    Ok(Box::new(session) as Box<dyn DriverSession>)
                }
                Engine::ClickHouse => {
                    let session = ClickHouseSession::connect_with_password(
                        &ClickHouseConnectConfig::new(
                            host,
                            port,
                            database.clone(),
                            user,
                            if tls_required {
                                ClickHouseTlsMode::Require
                            } else {
                                ClickHouseTlsMode::Disable
                            },
                            ClickHouseCompression::Lz4,
                        ),
                        password_opt,
                    );
                    session
                        .health_check()
                        .await
                        .map_err(|error| BridgeError::rejected("connect", error.to_string()))?;
                    Ok(Box::new(session) as Box<dyn DriverSession>)
                }
                Engine::Redis => {
                    let db = params.database.parse::<u32>().unwrap_or(0);
                    let mut security = RedisConnectionSecurity::new();
                    if !password.is_empty() || !params.user.is_empty() {
                        let username = if params.user.is_empty() {
                            None
                        } else {
                            Some(params.user.as_str())
                        };
                        security = security
                            .with_credentials(RedisCredentials::new(username, password.as_str()));
                    }
                    let session = RedisSession::connect(
                        &RedisConnectConfig::new(
                            host,
                            port,
                            db,
                            RedisProtocol::Resp3,
                            if tls_required {
                                RedisTlsMode::Require
                            } else {
                                RedisTlsMode::Disable
                            },
                        ),
                        security,
                    )
                    .await
                    .map_err(|error| BridgeError::rejected("connect", error.to_string()))?;
                    Ok(Box::new(session) as Box<dyn DriverSession>)
                }
            }
        })??;

        self.open_driver_session_inner(
            engine,
            session,
            saved_profile_id,
            Some(database),
            postgres_tool_connection,
        )
    }

    fn open_driver_session_inner(
        &self,
        engine: Engine,
        session: Box<dyn DriverSession>,
        saved_profile_id: Option<ProfileId>,
        database: Option<BoundedText>,
        postgres_tool_connection: Option<PostgresToolConnection>,
    ) -> Result<Vec<u8>, BridgeError> {
        self.ensure_runtime_inner()?;
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        if !inner.accepting {
            return Err(BridgeError::ShuttingDown);
        }
        let profile_id = saved_profile_id.unwrap_or_else(|| inner.ids.profile());
        let session_id = inner.ids.session();
        let context_id = inner.ids.context();

        let profile_scope = CommandScope::Profile(profile_id);
        if inner
            .service
            .core_mut()
            .scope_revision(profile_scope)
            .is_none()
        {
            inner
                .service
                .core_mut()
                .register_scope(profile_scope, Revision::INITIAL)
                .map_err(|error| BridgeError::rejected("register-profile", error.to_string()))?;
        }
        inner
            .service
            .core_mut()
            .register_scope(
                CommandScope::Session {
                    profile_id,
                    session_id,
                },
                Revision::INITIAL,
            )
            .map_err(|error| BridgeError::rejected("register-session-scope", error.to_string()))?;
        let op_scope = OperationScope::new(profile_id, session_id, context_id);
        inner
            .service
            .core_mut()
            .register_scope(CommandScope::Context(op_scope), Revision::INITIAL)
            .map_err(|error| BridgeError::rejected("register-context", error.to_string()))?;

        inner
            .service
            .register_session(session_id, session)
            .map_err(|error| BridgeError::rejected("register-session", error.to_string()))?;

        inner.sessions.insert(
            session_id,
            RegisteredSession {
                profile_id,
                session_id,
                context_id,
                engine,
                database: database.unwrap_or_else(|| {
                    BoundedText::copy_from_str(
                        match engine {
                            Engine::PostgreSql => "postgres",
                            Engine::ClickHouse => "default",
                            Engine::Redis => "0",
                        },
                        ByteLimit::new(16),
                    )
                    .expect("default database is bounded")
                }),
                postgres_tool_connection,
                context_revision: Revision::INITIAL,
            },
        );
        Ok(session_bytes(session_id))
    }

    fn submit_named_inner(
        &self,
        mut spec: SubmitSpec,
        bindings: Vec<BridgeQueryParameter>,
    ) -> Result<Vec<u8>, BridgeError> {
        if !spec.intent.eq_ignore_ascii_case("execute") {
            return Err(BridgeError::rejected(
                "named-parameter-intent",
                "named parameters require execute intent",
            ));
        }
        let statement = spec.statement.as_deref().ok_or_else(|| {
            BridgeError::rejected("named-parameter-statement", "statement is required")
        })?;
        let plan = rewrite_named_params(statement)
            .map_err(|error| BridgeError::rejected("named-parameters", error.to_string()))?;
        if plan.names.is_empty() {
            return Err(BridgeError::rejected(
                "named-parameters-empty",
                "statement has no named parameters",
            ));
        }
        if bindings.len() != plan.names.len() {
            return Err(BridgeError::rejected(
                "named-parameter-count",
                "every named parameter requires exactly one binding",
            ));
        }

        let engine = {
            let guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_ref().ok_or(BridgeError::RuntimeUnavailable)?;
            let session_id = session_from_bytes(&spec.session_id).map_err(|_| {
                BridgeError::rejected("bad-session-id", "session id must be 16 bytes")
            })?;
            inner
                .sessions
                .get(&session_id)
                .ok_or(BridgeError::UnknownSession)?
                .engine
        };
        if engine == Engine::Redis {
            return Err(BridgeError::rejected(
                "named-parameter-engine",
                "named SQL parameters are unavailable for Redis",
            ));
        }

        let mut by_name = BTreeMap::new();
        for binding in bindings {
            if binding.name.len() > 64 || binding.value.as_ref().is_some_and(|v| v.len() > 65_536) {
                return Err(BridgeError::rejected(
                    "named-parameter-limit",
                    "parameter name or value exceeds its byte limit",
                ));
            }
            let name = binding.name.clone();
            if by_name.insert(name, binding).is_some() {
                return Err(BridgeError::rejected(
                    "named-parameter-duplicate",
                    "parameter names must be unique",
                ));
            }
        }

        let mut parameters = Vec::with_capacity(plan.names.len());
        let mut kinds = Vec::with_capacity(plan.names.len());
        for name in &plan.names {
            let binding = by_name.remove(name).ok_or_else(|| {
                BridgeError::rejected("named-parameter-missing", "required parameter is missing")
            })?;
            let (value, clickhouse_type) = parse_bridge_query_parameter(binding)?;
            parameters.push(value);
            kinds.push(clickhouse_type);
        }
        if !by_name.is_empty() {
            return Err(BridgeError::rejected(
                "named-parameter-extra",
                "unexpected parameter name",
            ));
        }

        spec.statement = Some(if engine == Engine::ClickHouse {
            plan.render_with_placeholders(|index, _| format!("{{p{}:{}}}", index + 1, kinds[index]))
        } else {
            plan.sql
        });
        self.submit_inner_with_parameters(spec, parameters)
    }

    fn submit_inner(&self, spec: SubmitSpec) -> Result<Vec<u8>, BridgeError> {
        self.submit_inner_with_parameters(spec, Vec::new())
    }

    fn submit_inner_with_parameters(
        &self,
        spec: SubmitSpec,
        parameters: Vec<tablerock_engine::FilterValue>,
    ) -> Result<Vec<u8>, BridgeError> {
        self.ensure_runtime_inner()?;
        let session_id = session_from_bytes(&spec.session_id)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;

        let (operation_id, command, request, page_identity, driver) = {
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
            if !inner.accepting {
                return Err(BridgeError::ShuttingDown);
            }
            let registered = inner
                .sessions
                .get(&session_id)
                .ok_or(BridgeError::UnknownSession)?;
            let engine = registered.engine;
            let scope = OperationScope::new(
                registered.profile_id,
                registered.session_id,
                registered.context_id,
            );
            let expected = Revision::from_wire_u64(spec.expected_revision);
            if expected != registered.context_revision {
                return Err(BridgeError::rejected(
                    "revision-mismatch",
                    "expected revision does not match context",
                ));
            }
            let operation_id = inner.ids.operation();
            let result_id = match &spec.result_id {
                Some(bytes) => result_from_bytes(bytes).map_err(|_| {
                    BridgeError::rejected("bad-result-id", "result id must be 16 bytes")
                })?,
                None => inner.ids.result(),
            };
            let page_identity = PageIdentity::new(result_id, Revision::INITIAL, engine);
            let row_count = spec.row_count.unwrap_or(500).max(1);
            let budget = CommandBudget::new(60_000, 1_024, 16 * 1024 * 1024, row_count)
                .map_err(|error| BridgeError::rejected("budget", error.to_string()))?
                .validate(
                    CommandBudgetLimits::new(60_000, 1_024, 16 * 1024 * 1024, 10_000).map_err(
                        |error| BridgeError::rejected("budget-limits", error.to_string()),
                    )?,
                )
                .map_err(|error| BridgeError::rejected("budget-validate", error.to_string()))?;

            let intent_name = spec.intent.to_ascii_lowercase();
            if intent_name == "fetch_page" {
                // Serve from ResultStore without spawning a driver task.
                let key = PageKey::new(result_id, Revision::INITIAL, spec.start_row.unwrap_or(0));
                let page = inner
                    .results
                    .get(key)
                    .ok_or(BridgeError::UnknownPage)?
                    .clone();
                let encoded = page.encode_v1();
                let op_bytes = operation_bytes(operation_id);
                inner.push_event(BridgeEventRecord {
                    sequence: 0, // filled by push_event
                    operation_id: op_bytes.clone(),
                    kind: "page".into(),
                    outcome: None,
                    rows: Some(u64::from(page.envelope().row_count())),
                    bytes: Some(page.envelope().arena_byte_len()),
                    page_bytes: Some(encoded),
                });
                return Ok(op_bytes);
            }

            let pending_history =
                matches!(intent_name.as_str(), "execute" | "explain").then(|| {
                    let database_name = inner
                        .persistence
                        .as_ref()
                        .and_then(|actor| actor.get_profile(registered.profile_id).ok().flatten())
                        .and_then(|profile| {
                            profile
                                .connection()
                                .properties()
                                .literal(ProfileProperty::DefaultContext)
                                .map(str::to_owned)
                        })
                        .unwrap_or_else(|| match engine {
                            Engine::PostgreSql => "postgres".into(),
                            Engine::ClickHouse => "default".into(),
                            Engine::Redis => "0".into(),
                        });
                    HistoryAppend {
                        engine,
                        database_name,
                        schema_name: None,
                        statement_text: spec.statement.clone().unwrap_or_else(|| "select 1".into()),
                        outcome: HistoryOutcomeClass::Unknown,
                        retention: inner.history_retention,
                    }
                });

            let (intent, request) = match intent_name.as_str() {
                "probe" | "execute" | "browse_object" | "explain" => {
                    let mut statement = spec.statement.clone().unwrap_or_else(|| "select 1".into());
                    if intent_name == "explain" {
                        let body = statement.trim();
                        if body.is_empty() {
                            return Err(BridgeError::rejected(
                                "explain-empty",
                                "EXPLAIN needs SQL in the active editor",
                            ));
                        }
                        if engine == Engine::Redis {
                            return Err(BridgeError::rejected(
                                "explain-unsupported",
                                "EXPLAIN is unsupported for Redis",
                            ));
                        }
                        statement = if starts_with_explain_keyword(body) {
                            body.to_owned()
                        } else if engine == Engine::ClickHouse {
                            format!("EXPLAIN {body}")
                        } else {
                            format!("EXPLAIN (FORMAT TEXT) {body}")
                        };
                    }
                    let text = StatementText::new(statement)
                        .map_err(|error| BridgeError::rejected("statement", error.to_string()))?;
                    let limits = default_page_limits();
                    let max_cell_bytes = 64 * 1024;
                    let request = match (engine, intent_name.as_str()) {
                        (Engine::PostgreSql, "execute" | "browse_object" | "explain") => {
                            DriverPageRequest::PostgreSqlStatement {
                                statement: text.clone(),
                                parameters: parameters.clone(),
                                limits,
                                max_cell_bytes,
                            }
                        }
                        (Engine::PostgreSql, _) => DriverPageRequest::PostgreSqlProbe {
                            query: PostgresProbeQuery::BoundedSeries,
                            limits,
                            max_cell_bytes,
                        },
                        (Engine::ClickHouse, "execute" | "browse_object" | "explain") => {
                            DriverPageRequest::ClickHouseStatement {
                                statement: text.clone(),
                                parameters: parameters.clone(),
                                query_id: BoundedText::copy_from_str(
                                    &format!("bridge-{}", page_identity.result_id()),
                                    ByteLimit::new(128),
                                )
                                .map_err(|error| {
                                    BridgeError::rejected("query-id", error.to_string())
                                })?,
                                limits,
                                max_cell_bytes,
                            }
                        }
                        (Engine::ClickHouse, _) => DriverPageRequest::ClickHouseProbe {
                            query: ClickHouseProbeQuery::TypedValues,
                            query_id: BoundedText::copy_from_str(
                                &format!("bridge-probe-{}", page_identity.result_id()),
                                ByteLimit::new(128),
                            )
                            .map_err(|error| {
                                BridgeError::rejected("query-id", error.to_string())
                            })?,
                            limits,
                            max_cell_bytes,
                        },
                        (Engine::Redis, _) => DriverPageRequest::RedisKeyScan {
                            limits,
                            max_cell_bytes,
                            scan_count: 16,
                            max_scan_rounds: 128,
                            match_pattern: None,
                        },
                    };
                    (CommandIntent::Execute { statement: text }, request)
                }
                other => {
                    return Err(BridgeError::rejected(
                        "unknown-intent",
                        format!("unsupported intent {other}"),
                    ));
                }
            };
            let _ = PageRequest::new(result_id, Revision::INITIAL, 0, row_count);

            let command = CommandEnvelope::new(
                tablerock_core::RequestId::from_parts(inner.ids.parts())
                    .map_err(|error| BridgeError::rejected("request-id", error.to_string()))?,
                CommandScope::Context(scope),
                expected,
                budget,
                None,
                intent,
            )
            .map_err(|error| BridgeError::rejected("command", error.to_string()))?;

            let driver = inner
                .service
                .session(session_id)
                .ok_or(BridgeError::UnknownSession)?;
            inner.operation_results.insert(operation_id, page_identity);
            if let Some(history) = pending_history {
                inner.operation_history.insert(operation_id, history);
            }
            (operation_id, command, request, page_identity, driver)
        };

        // Submit requires &mut service — re-lock and call.
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        self.runtime.block_on(async {
            inner
                .service
                .submit(operation_id, command, driver, request, page_identity)
                .await
                .map_err(|error| BridgeError::rejected("submit", error.to_string()))
        })??;
        Ok(operation_bytes(operation_id))
    }

    fn pump_inner(&self, operation_id_bytes: Vec<u8>) -> Result<(), BridgeError> {
        let operation_id = operation_from_bytes(&operation_id_bytes).map_err(|_| {
            BridgeError::rejected("bad-operation-id", "operation id must be 16 bytes")
        })?;
        self.ensure_runtime_inner()?;
        loop {
            // Never hold the coarse bridge mutex for an unbounded driver wait:
            // cancel() needs the same service lock to dispatch by operation ID.
            // A short timeout bounds lock ownership; yielding outside the lock
            // lets a concurrent cancellation acquire it before the next poll.
            let (timed_out, update) = {
                let mut guard = self
                    .inner
                    .lock()
                    .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
                let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
                match self.runtime.block_on(async {
                    tokio::time::timeout(
                        Duration::from_millis(10),
                        inner.service.next_update(operation_id),
                    )
                    .await
                }) {
                    Ok(Ok(Ok(update))) => (false, update),
                    Ok(Ok(Err(error))) => {
                        return Err(BridgeError::rejected("pump", error.to_string()));
                    }
                    Ok(Err(_elapsed)) => (true, None),
                    Err(error) => return Err(error),
                }
            };
            if timed_out {
                std::thread::yield_now();
                continue;
            }
            let Some(update) = update else {
                break;
            };
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
            let terminal = inner.apply_update(operation_id, update)?;
            if terminal {
                inner
                    .service
                    .retire(operation_id)
                    .map_err(|error| BridgeError::rejected("retire", error.to_string()))?;
                break;
            }
        }
        Ok(())
    }

    fn next_events_inner(
        &self,
        cursor: u64,
        maximum: u32,
    ) -> Result<BridgeEventBatch, BridgeError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        if maximum == 0 {
            return Err(BridgeError::rejected("batch", "maximum must be nonzero"));
        }
        let maximum = maximum.min(MAX_EVENT_BATCH) as usize;
        if cursor > inner.next_sequence {
            return Err(BridgeError::FutureCursor);
        }
        if cursor < inner.first_sequence {
            return Ok(BridgeEventBatch {
                next_cursor: inner.next_sequence,
                events: Vec::new(),
                resync_required: true,
            });
        }
        let skip = (cursor - inner.first_sequence) as usize;
        let events: Vec<_> = inner
            .events
            .iter()
            .skip(skip)
            .take(maximum)
            .cloned()
            .collect();
        let next_cursor = events
            .last()
            .map(|event| event.sequence.saturating_add(1))
            .unwrap_or(cursor);
        Ok(BridgeEventBatch {
            next_cursor,
            events,
            resync_required: false,
        })
    }

    fn fetch_page_inner(
        &self,
        result_id: Vec<u8>,
        start_row: u64,
        revision: u64,
    ) -> Result<Vec<u8>, BridgeError> {
        let result_id = result_from_bytes(&result_id)
            .map_err(|_| BridgeError::rejected("bad-result-id", "result id must be 16 bytes"))?;
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        let key = PageKey::new(result_id, Revision::from_wire_u64(revision), start_row);
        let page = inner.results.get(key).ok_or(BridgeError::UnknownPage)?;
        Ok(page.encode_v1())
    }

    fn cancel_inner(&self, operation_id: Vec<u8>) -> Result<CancelOutcome, BridgeError> {
        let operation_id = operation_from_bytes(&operation_id).map_err(|_| {
            BridgeError::rejected("bad-operation-id", "operation id must be 16 bytes")
        })?;
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        let outcome = inner
            .service
            .cancel(operation_id)
            .map_err(|error| BridgeError::rejected("cancel", error.to_string()))?;
        Ok(CancelOutcome {
            core: format!("{:?}", outcome.core),
            runtime: outcome.runtime.map(|value| format!("{value:?}")),
        })
    }

    fn shutdown_inner(
        &self,
        cancel_active: bool,
        deadline_ms: u64,
    ) -> Result<ShutdownOutcome, BridgeError> {
        self.ensure_runtime_inner()?;
        let mode = if cancel_active {
            ShutdownMode::CancelActive
        } else {
            ShutdownMode::Graceful
        };
        let initial_tool_active = {
            let mut tasks = self.postgres_tools.lock().map_err(|_| {
                BridgeError::rejected("postgres-tool-lock", "tool registry mutex poisoned")
            })?;
            let mut active = 0_u32;
            for task in tasks.values_mut() {
                if task.phase == "running" || task.phase == "cancel_requested" {
                    active = active.saturating_add(1);
                    if cancel_active && task.phase == "running" {
                        let _ = task.cancel.send(true);
                        task.phase = "cancel_requested".into();
                        task.summary = "Cancellation requested".into();
                    }
                }
            }
            active
        };
        let initial_subscription_active = {
            let mut tasks = self.redis_subscriptions.lock().map_err(|_| {
                BridgeError::rejected(
                    "redis-subscription-lock",
                    "subscription registry mutex poisoned",
                )
            })?;
            let mut active = 0_u32;
            for task in tasks.values_mut() {
                if task.phase == "connecting"
                    || task.phase == "listening"
                    || task.phase == "cancel_requested"
                {
                    active = active.saturating_add(1);
                    if cancel_active && task.phase != "cancel_requested" {
                        let _ = task.cancel.send(true);
                        task.phase = "cancel_requested".into();
                        task.summary = "Cancellation requested".into();
                    }
                }
            }
            active
        };
        let initial_import_active = {
            let mut tasks = self.csv_imports.lock().map_err(|_| {
                BridgeError::rejected("csv-import-lock", "import task registry poisoned")
            })?;
            let mut active = 0_u32;
            for task in tasks.values_mut() {
                if task.phase == "running" || task.phase == "cancel_requested" {
                    active = active.saturating_add(1);
                    if cancel_active && task.phase == "running" {
                        task.control.request_cancel();
                        task.phase = "cancel_requested".into();
                        task.summary =
                            "Cancellation requested; waiting for the current row boundary".into();
                    }
                }
            }
            active
        };
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        inner.accepting = false;
        let outcome = inner
            .service
            .begin_shutdown(mode)
            .map_err(|error| BridgeError::rejected("shutdown", error.to_string()))?;
        let initial_active = match outcome.core {
            tablerock_core::ShutdownOutcome::Draining { active_operations } => active_operations,
            tablerock_core::ShutdownOutcome::Stopped
            | tablerock_core::ShutdownOutcome::AlreadyStopped => 0,
        }
        .saturating_add(initial_tool_active)
        .saturating_add(initial_subscription_active)
        .saturating_add(initial_import_active);
        let deadline = Duration::from_millis(deadline_ms);
        let started = Instant::now();
        while inner.service.core().active_operations() > 0 && started.elapsed() < deadline {
            let operation_ids = inner.service.active_operation_ids();
            if operation_ids.is_empty() {
                break;
            }
            for operation_id in operation_ids {
                let remaining = deadline.saturating_sub(started.elapsed());
                if remaining.is_zero() {
                    break;
                }
                let wait = remaining.min(Duration::from_millis(10));
                let update = self.runtime.block_on(async {
                    tokio::time::timeout(wait, inner.service.next_update(operation_id)).await
                })?;
                let Ok(update) = update else { continue };
                let update = update
                    .map_err(|error| BridgeError::rejected("shutdown-drain", error.to_string()))?;
                if let Some(update) = update {
                    let terminal = inner.apply_update(operation_id, update)?;
                    if terminal {
                        inner.service.retire(operation_id).map_err(|error| {
                            BridgeError::rejected("shutdown-retire", error.to_string())
                        })?;
                    }
                }
            }
        }
        while started.elapsed() < deadline {
            let tool_active = self
                .postgres_tools
                .lock()
                .map_err(|_| {
                    BridgeError::rejected("postgres-tool-lock", "tool registry mutex poisoned")
                })?
                .values()
                .any(|task| task.phase == "running" || task.phase == "cancel_requested");
            let subscription_active = self
                .redis_subscriptions
                .lock()
                .map_err(|_| {
                    BridgeError::rejected(
                        "redis-subscription-lock",
                        "subscription registry mutex poisoned",
                    )
                })?
                .values()
                .any(|task| {
                    task.phase == "connecting"
                        || task.phase == "listening"
                        || task.phase == "cancel_requested"
                });
            let import_active = self
                .csv_imports
                .lock()
                .map_err(|_| {
                    BridgeError::rejected("csv-import-lock", "import task registry poisoned")
                })?
                .values()
                .any(|task| task.phase == "running" || task.phase == "cancel_requested");
            if !tool_active && !subscription_active && !import_active {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let tool_active = self
            .postgres_tools
            .lock()
            .map_err(|_| {
                BridgeError::rejected("postgres-tool-lock", "tool registry mutex poisoned")
            })?
            .values()
            .filter(|task| task.phase == "running" || task.phase == "cancel_requested")
            .count() as u32;
        let subscription_active = self
            .redis_subscriptions
            .lock()
            .map_err(|_| {
                BridgeError::rejected(
                    "redis-subscription-lock",
                    "subscription registry mutex poisoned",
                )
            })?
            .values()
            .filter(|task| {
                task.phase == "connecting"
                    || task.phase == "listening"
                    || task.phase == "cancel_requested"
            })
            .count() as u32;
        let import_active = self
            .csv_imports
            .lock()
            .map_err(|_| BridgeError::rejected("csv-import-lock", "import task registry poisoned"))?
            .values()
            .filter(|task| task.phase == "running" || task.phase == "cancel_requested")
            .count() as u32;
        let active = inner
            .service
            .core()
            .active_operations()
            .saturating_add(tool_active)
            .saturating_add(subscription_active)
            .saturating_add(import_active);
        let core = if active == 0 {
            let _ = self.runtime.block_on(inner.service.complete_shutdown());
            "Stopped".to_owned()
        } else {
            format!("Draining {{ active_operations: {active} }}")
        };
        if active == 0 {
            drop(guard);
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            *guard = None;
            let _ = self.runtime.shutdown();
        }
        Ok(ShutdownOutcome {
            core: if initial_active == 0 {
                format!("{:?}", outcome.core)
            } else {
                core
            },
            active_operations: active,
        })
    }
}

fn bounded_catalog_name(name: &str) -> Result<BoundedText, BridgeError> {
    BoundedText::copy_from_str(name, ByteLimit::new(1_024))
        .map_err(|error| BridgeError::rejected("catalog-name", error.to_string()))
}

fn environment_label(environment: &EnvironmentTag) -> String {
    match environment {
        EnvironmentTag::Production => "Production".into(),
        EnvironmentTag::Staging => "Staging".into(),
        EnvironmentTag::Development => "Development".into(),
        EnvironmentTag::Testing => "Testing".into(),
        EnvironmentTag::Custom(_) => environment.custom_label().unwrap_or("Custom").to_owned(),
    }
}

fn bridge_activity_error(code: &str, error: tablerock_engine::AdapterError) -> BridgeError {
    let message = if error.class() == AdapterFailureClass::PermissionDenied {
        "permission denied"
    } else {
        "PostgreSQL activity operation failed"
    };
    BridgeError::rejected(code, message)
}

fn postgres_tool_kind(kind: &str) -> Result<(&'static str, bool), BridgeError> {
    match kind {
        "dump" => Ok(("pg_dump", false)),
        "restore" => Ok(("pg_restore", true)),
        _ => Err(BridgeError::rejected(
            "postgres-tool-kind",
            "tool kind must be dump or restore",
        )),
    }
}

fn probe_postgres_tool_inner(
    kind: String,
    explicit_path: Option<String>,
) -> Result<BridgePostgresToolProbe, BridgeError> {
    let (tool_name, _) = postgres_tool_kind(&kind)?;
    let explicit = explicit_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    Ok(match discover_tool(tool_name, explicit) {
        ToolStatus::Found { path, version } => BridgePostgresToolProbe {
            kind,
            available: true,
            path: Some(path.display().to_string()),
            version: Some(version.clone()),
            summary: version,
        },
        ToolStatus::Missing { .. } => BridgePostgresToolProbe {
            kind,
            available: false,
            path: None,
            version: None,
            summary: format!("{tool_name} not found"),
        },
        ToolStatus::VersionProbeFailed { path, .. } => BridgePostgresToolProbe {
            kind,
            available: false,
            path: Some(path.display().to_string()),
            version: None,
            summary: "Version probe failed".into(),
        },
    })
}

fn bridge_profile_item(
    item: &tablerock_core::ProfileListItem,
    connected: bool,
) -> BridgeProfileItem {
    BridgeProfileItem {
        id_bytes: item.id().to_bytes().to_vec(),
        revision: item.revision().get(),
        name: item.name().as_str().to_owned(),
        engine: engine_label(item.engine()).to_owned(),
        group: item.group().map(|group| group.as_str().to_owned()),
        favorite: item.favorite(),
        saved_order: item.saved_order(),
        host: item.endpoint().host().literal_value().map(str::to_owned),
        port: item.endpoint().port().literal_value().map(str::to_owned),
        context: item
            .endpoint()
            .context()
            .and_then(ProfileEndpointPart::literal_value)
            .map(str::to_owned),
        safety_mode: match item.safety_mode() {
            ProfileSafetyMode::ReadOnly => "read_only",
            ProfileSafetyMode::ConfirmWrites => "confirm_writes",
        }
        .to_owned(),
        environment: item.environment().map(environment_label),
        production_warning: item
            .environment()
            .is_some_and(EnvironmentTag::is_production_warning),
        dangerous_plaintext: item.sources().has_dangerous_plaintext(),
        connected,
    }
}

fn decode_profile_id(bytes: &[u8]) -> Result<ProfileId, BridgeError> {
    let bytes = <[u8; 16]>::try_from(bytes)
        .map_err(|_| BridgeError::rejected("profile-id", "profile id must be 16 bytes"))?;
    ProfileId::from_bytes(bytes)
        .map_err(|_| BridgeError::rejected("profile-id", "invalid profile id"))
}

fn profile_to_bridge_draft(profile: &ProfileAggregate) -> Result<BridgeProfileDraft, BridgeError> {
    let connection = profile.connection();
    let literal = |property| {
        connection
            .properties()
            .binding(property)
            .and_then(ProfilePropertyBinding::literal_value)
            .unwrap_or_default()
            .to_owned()
    };
    let (password_source, password_value, has_stored_password) = connection
        .properties()
        .binding(ProfileProperty::Password)
        .and_then(ProfilePropertyBinding::secret_source)
        .map(|source| match source.kind() {
            SecretSourceKind::PromptOnConnect => ("prompt", String::new(), false),
            SecretSourceKind::HostEnvironment(reference) => {
                ("environment", reference.as_str().to_owned(), false)
            }
            SecretSourceKind::OnePassword(reference) => {
                ("onepassword", reference.to_compact_wire(), false)
            }
            SecretSourceKind::DangerousPlaintext(_) => ("dangerous_plaintext", String::new(), true),
            SecretSourceKind::Keychain(_) => ("keychain", String::new(), true),
        })
        .unwrap_or(("prompt", String::new(), false));
    let tls_mode = match connection.tls_policy() {
        TlsPolicy::Disabled => "off",
        TlsPolicy::VerifySystemRoots => "verify_ca",
        TlsPolicy::VerifyCustomCa => "verify_full",
        TlsPolicy::DangerousAcceptInvalidCertificate(_) => "dangerous",
    };
    Ok(BridgeProfileDraft {
        id_bytes: Some(connection.id().to_bytes().to_vec()),
        revision: connection.revision().get(),
        engine: engine_label(connection.engine()).to_owned(),
        name: connection.name().as_str().to_owned(),
        group: profile
            .organization()
            .group()
            .map(ProfileGroupName::as_str)
            .unwrap_or_default()
            .to_owned(),
        environment: profile
            .organization()
            .environment()
            .map(environment_label)
            .unwrap_or_default(),
        host: literal(ProfileProperty::Host),
        port: literal(ProfileProperty::Port),
        database: literal(ProfileProperty::DefaultContext),
        username: literal(ProfileProperty::Username),
        password_source: password_source.to_owned(),
        password_value,
        password_reference: connection
            .properties()
            .binding(ProfileProperty::Password)
            .and_then(ProfilePropertyBinding::secret_source)
            .and_then(|source| match source.kind() {
                SecretSourceKind::Keychain(reference) => Some(reference.bytes().to_vec()),
                _ => None,
            }),
        has_stored_password,
        plaintext_acknowledged: has_stored_password,
        tls_mode: tls_mode.to_owned(),
        safety_mode: match connection.safety_mode() {
            ProfileSafetyMode::ReadOnly => "read_only",
            ProfileSafetyMode::ConfirmWrites => "confirm_writes",
        }
        .to_owned(),
    })
}

fn bridge_draft_to_profile(
    draft: &BridgeProfileDraft,
    id: ProfileId,
    existing: Option<&ProfileAggregate>,
) -> Result<ProfileAggregate, BridgeError> {
    let rejected = |code: &'static str, error: String| BridgeError::rejected(code, error);
    let text = |value: &str, maximum| {
        BoundedText::copy_from_str(value, ByteLimit::new(maximum))
            .map_err(|error| rejected("profile-field", error.to_string()))
    };
    let engine = match draft.engine.as_str() {
        "postgresql" => Engine::PostgreSql,
        "clickhouse" => Engine::ClickHouse,
        "redis" => Engine::Redis,
        _ => return Err(BridgeError::rejected("profile-engine", "unknown engine")),
    };
    let revision = if existing.is_some() {
        Revision::from_wire_u64(draft.revision)
            .checked_next()
            .map_err(|error| rejected("profile-revision", error.to_string()))?
    } else {
        Revision::INITIAL
    };
    let mut bindings = vec![
        ProfilePropertyBinding::literal(ProfileProperty::Host, text(draft.host.trim(), 128)?)
            .map_err(|error| rejected("profile-host", error.to_string()))?,
        ProfilePropertyBinding::literal(ProfileProperty::Port, text(draft.port.trim(), 16)?)
            .map_err(|error| rejected("profile-port", error.to_string()))?,
    ];
    for (property, value) in [
        (ProfileProperty::DefaultContext, draft.database.trim()),
        (ProfileProperty::Username, draft.username.trim()),
    ] {
        if !value.is_empty() {
            bindings.push(
                ProfilePropertyBinding::literal(property, text(value, 128)?)
                    .map_err(|error| rejected("profile-field", error.to_string()))?,
            );
        }
    }
    let password_kind = match draft.password_source.as_str() {
        "prompt" => SecretSourceKind::PromptOnConnect,
        "environment" => SecretSourceKind::HostEnvironment(
            EnvironmentReference::parse(draft.password_value.trim())
                .map_err(|error| rejected("profile-password", error.to_string()))?,
        ),
        "onepassword" => SecretSourceKind::OnePassword(
            OnePasswordReference::from_compact_wire(draft.password_value.trim())
                .map_err(|error| rejected("profile-password", error.to_string()))?,
        ),
        "dangerous_plaintext" => {
            if !draft.plaintext_acknowledged {
                return Err(BridgeError::rejected(
                    "profile-password",
                    "plaintext password acknowledgement required",
                ));
            }
            if draft.password_value.is_empty() {
                return Err(BridgeError::rejected(
                    "profile-password",
                    "re-enter the stored plaintext password before saving",
                ));
            }
            SecretSourceKind::DangerousPlaintext(
                DangerousPlaintext::new(
                    draft.password_value.as_bytes().to_vec(),
                    PlaintextAcknowledgement::LocalTestingOnly,
                )
                .map_err(|error| rejected("profile-password", error.to_string()))?,
            )
        }
        "keychain" => {
            let reference = draft.password_reference.as_deref().or_else(|| {
                existing
                    .and_then(|profile| {
                        profile
                            .connection()
                            .properties()
                            .binding(ProfileProperty::Password)
                    })
                    .and_then(ProfilePropertyBinding::secret_source)
                    .and_then(|source| match source.kind() {
                        SecretSourceKind::Keychain(reference) => Some(reference.bytes()),
                        _ => None,
                    })
            });
            let reference = reference.ok_or_else(|| {
                BridgeError::rejected("profile-password", "Keychain password reference required")
            })?;
            SecretSourceKind::Keychain(
                KeychainReference::new(
                    tablerock_core::BoundedBytes::copy_from_slice(
                        reference,
                        ByteLimit::new(KeychainReference::MAX_BYTES),
                    )
                    .map_err(|error| rejected("profile-password", error.to_string()))?,
                )
                .map_err(|error| rejected("profile-password", error.to_string()))?,
            )
        }
        _ => {
            return Err(BridgeError::rejected(
                "profile-password",
                "unknown password source",
            ));
        }
    };
    bindings.push(ProfilePropertyBinding::secret(
        ProfileProperty::Password,
        SecretSource::new(password_kind),
    ));
    let properties = ProfilePropertySet::new(bindings)
        .map_err(|error| rejected("profile-properties", error.to_string()))?;
    let tls = match draft.tls_mode.as_str() {
        "off" => TlsPolicy::Disabled,
        "verify_ca" => TlsPolicy::VerifySystemRoots,
        "verify_full" => TlsPolicy::VerifyCustomCa,
        _ => return Err(BridgeError::rejected("profile-tls", "unknown TLS mode")),
    };
    let safety = match draft.safety_mode.as_str() {
        "read_only" => ProfileSafetyMode::ReadOnly,
        "confirm_writes" => ProfileSafetyMode::ConfirmWrites,
        _ => {
            return Err(BridgeError::rejected(
                "profile-safety",
                "unknown safety mode",
            ));
        }
    };
    let connection = ProfileConnectionSnapshot::new(
        ProfileIdentity::new(
            id,
            revision,
            engine,
            ProfileName::new(text(draft.name.trim(), ProfileName::MAX_BYTES)?)
                .map_err(|error| rejected("profile-name", error.to_string()))?,
        ),
        properties,
        ProfilePolicy::new(
            tls,
            safety,
            existing
                .map(|profile| profile.connection().limits())
                .unwrap_or(ProfileLimits::new(10_000, 30_000, 5_000, 16 * 1024 * 1024).unwrap()),
        ),
    )
    .map_err(|error| rejected("profile", error.to_string()))?;
    let group = if draft.group.trim().is_empty() {
        None
    } else {
        Some(
            ProfileGroupName::new(text(draft.group.trim(), ProfileGroupName::MAX_BYTES)?)
                .map_err(|error| rejected("profile-group", error.to_string()))?,
        )
    };
    let environment = parse_bridge_environment(&draft.environment)?;
    let old_organization = existing.map(ProfileAggregate::organization);
    let organization = ProfileOrganization::new(
        group,
        old_organization
            .map(|organization| organization.tags().to_vec())
            .unwrap_or_default(),
        old_organization.is_some_and(ProfileOrganization::favorite),
        old_organization
            .map(ProfileOrganization::order)
            .unwrap_or(0),
        environment,
    )
    .map_err(|error| rejected("profile-organization", error.to_string()))?;
    ProfileAggregate::new(
        connection,
        ProfileDurability::Saved,
        organization,
        existing.map(ProfileAggregate::preferences).unwrap_or(
            ProfilePreferences::new(ReconnectPreference::BoundedAutomatic, true, 250).unwrap(),
        ),
    )
    .map(|profile| match existing {
        Some(old) => profile.with_startup_actions(old.startup_actions().clone()),
        None => profile,
    })
    .map_err(|error| rejected("profile", error.to_string()))
}

fn parse_bridge_environment(raw: &str) -> Result<Option<EnvironmentTag>, BridgeError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    Ok(Some(match raw.to_ascii_lowercase().as_str() {
        "production" | "prod" => EnvironmentTag::Production,
        "staging" => EnvironmentTag::Staging,
        "development" | "dev" => EnvironmentTag::Development,
        "testing" | "test" => EnvironmentTag::Testing,
        label => EnvironmentTag::Custom(
            ProfileTag::new(
                BoundedText::copy_from_str(label, ByteLimit::new(ProfileTag::MAX_BYTES)).map_err(
                    |error| BridgeError::rejected("profile-environment", error.to_string()),
                )?,
            )
            .map_err(|error| BridgeError::rejected("profile-environment", error.to_string()))?,
        ),
    }))
}

fn validate_bridge_group_name(raw: &str) -> Result<String, BridgeError> {
    let name = raw.trim();
    let bounded = BoundedText::copy_from_str(name, ByteLimit::new(ProfileGroupName::MAX_BYTES))
        .map_err(|error| BridgeError::rejected("profile-group", error.to_string()))?;
    ProfileGroupName::new(bounded)
        .map_err(|error| BridgeError::rejected("profile-group", error.to_string()))?;
    Ok(name.to_owned())
}

fn bridge_sql_path(raw: &str) -> Result<PathBuf, BridgeError> {
    if raw.len() > 4_096 {
        return Err(BridgeError::rejected(
            "sql-file-path",
            "SQL file path exceeds 4096 bytes",
        ));
    }
    let path = Path::new(raw);
    if !path.is_absolute() || path.extension().and_then(|value| value.to_str()) != Some("sql") {
        return Err(BridgeError::rejected(
            "sql-file-path",
            "SQL file path must be absolute and end in .sql",
        ));
    }
    Ok(path.to_path_buf())
}

fn bridge_sql_file(text: String, facts: SqlFileFacts) -> BridgeSqlFile {
    BridgeSqlFile {
        path: facts.path.to_string_lossy().into_owned(),
        statement_text: text,
        modified_nanos: facts
            .mtime
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .and_then(|value| u64::try_from(value.as_nanos()).ok()),
        len: facts.len,
    }
}

fn set_redis_subscription_terminal(
    tasks: &Mutex<BTreeMap<OperationId, RedisSubscriptionTask>>,
    operation_id: OperationId,
    phase: &str,
    summary: &str,
) {
    if let Ok(mut tasks) = tasks.lock()
        && let Some(task) = tasks.get_mut(&operation_id)
    {
        task.phase = phase.into();
        task.summary = summary.into();
    }
}

fn render_redis_subscription_cell(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => {
            const HEX: &[u8; 16] = b"0123456789abcdef";
            let mut rendered = String::with_capacity(2 + bytes.len().saturating_mul(2));
            rendered.push_str("0x");
            for byte in bytes {
                rendered.push(char::from(HEX[(byte >> 4) as usize]));
                rendered.push(char::from(HEX[(byte & 0x0f) as usize]));
            }
            rendered
        }
    }
}

struct StreamedCsvApplyOutcome {
    phase: String,
    completed_rows: u64,
    applied_rows: u64,
    conflict_rows: u64,
    failed_rows: u64,
    errors: Vec<String>,
    errors_truncated: bool,
    summary: String,
}

async fn apply_streamed_csv_import(
    driver: Arc<dyn DriverSession>,
    review: CsvImportReviewEntry,
    operation_id: OperationId,
    control: MutationApplyControl,
) -> Result<StreamedCsvApplyOutcome, ()> {
    let frozen_path = review.frozen_path.clone();
    let handle = tokio::runtime::Handle::current();
    let total_rows = review.row_count;
    let mut completed_rows = 0_u64;
    let mut applied_rows = 0_u64;
    let mut conflict_rows = 0_u64;
    let mut failed_rows = 0_u64;
    let mut errors = Vec::new();
    let mut errors_truncated = false;
    let mut batch_number = 0_u64;
    let mut fatal = false;
    let mut unknown = false;

    let scan = tokio::task::block_in_place(|| {
        stream_csv_batches(
            &frozen_path,
            csv_stream_limits().map_err(|_| ())?,
            |_, rows, _| {
                if rows.is_empty() {
                    return true;
                }
                if control.cancel_requested() || fatal || unknown {
                    return false;
                }
                let table = CsvTable {
                    headers: review.mapped_columns.clone(),
                    rows: rows.to_vec(),
                };
                let changes = match csv_to_typed_insert_changes(
                    &table,
                    &review.value_types,
                    CSV_IMPORT_MAX_CELL_BYTES as u64,
                ) {
                    Ok(changes) => changes,
                    Err(_) => {
                        fatal = true;
                        return false;
                    }
                };
                if validate_insert_batch_size(&changes, CSV_IMPORT_BATCH_ROWS as u32).is_err() {
                    fatal = true;
                    return false;
                }
                let mutation_id =
                    match derived_import_id::<MutationId>(operation_id, batch_number, 1) {
                        Ok(id) => id,
                        Err(()) => {
                            fatal = true;
                            return false;
                        }
                    };
                let batch_token =
                    match derived_import_id::<ReviewTokenId>(operation_id, batch_number, 2) {
                        Ok(id) => id,
                        Err(()) => {
                            fatal = true;
                            return false;
                        }
                    };
                let limits = match MutationPlanLimits::new(
                    CSV_IMPORT_BATCH_ROWS as u32,
                    1_024,
                    CSV_IMPORT_MAX_CELL_BYTES as u64,
                    8 * 1024 * 1024,
                    60_000,
                ) {
                    Ok(limits) => limits,
                    Err(_) => {
                        fatal = true;
                        return false;
                    }
                };
                let plan = match MutationPlan::new(
                    mutation_id,
                    review.scope,
                    review.revision,
                    review.target.clone(),
                    changes,
                    limits,
                ) {
                    Ok(plan) => plan,
                    Err(_) => {
                        fatal = true;
                        return false;
                    }
                };
                let authorized = match plan
                    .review(batch_token, 1, 60_001)
                    .and_then(|reviewed| reviewed.authorize(2, review.scope, review.revision))
                {
                    Ok(authorized) => authorized,
                    Err(_) => {
                        fatal = true;
                        return false;
                    }
                };
                let base = completed_rows;
                let mapped_control = {
                    let global = control.clone();
                    control.map_progress(move |completed, _| {
                        global.report(base.saturating_add(completed), total_rows);
                    })
                };
                let outcome = match handle.block_on(
                    driver.apply_authorized_mutation_controlled(authorized, mapped_control),
                ) {
                    Ok(outcome) => outcome,
                    Err(error) => {
                        if matches!(
                            error.class(),
                            AdapterFailureClass::Connection
                                | AdapterFailureClass::Timeout
                                | AdapterFailureClass::Protocol
                                | AdapterFailureClass::CancellationTransport
                                | AdapterFailureClass::ServerCancelled
                                | AdapterFailureClass::WriteOutcomeUnknown
                        ) {
                            unknown = true;
                        } else {
                            fatal = true;
                        }
                        return false;
                    }
                };
                let rolled_back = matches!(
                    outcome.transaction,
                    tablerock_engine::MutationTransactionState::RolledBack
                );
                if matches!(
                    outcome.transaction,
                    tablerock_engine::MutationTransactionState::Unknown
                ) {
                    unknown = true;
                }
                for change in &outcome.changes {
                    match change {
                        tablerock_engine::MutationChangeOutcome::Applied { .. } => {
                            if !rolled_back {
                                applied_rows = applied_rows.saturating_add(1);
                            }
                        }
                        tablerock_engine::MutationChangeOutcome::Conflict { index, .. } => {
                            conflict_rows = conflict_rows.saturating_add(1);
                            push_import_error(
                                &mut errors,
                                &mut errors_truncated,
                                base.saturating_add(*index as u64).saturating_add(2),
                                "conflict",
                            );
                        }
                        tablerock_engine::MutationChangeOutcome::Failed { index, .. } => {
                            failed_rows = failed_rows.saturating_add(1);
                            push_import_error(
                                &mut errors,
                                &mut errors_truncated,
                                base.saturating_add(*index as u64).saturating_add(2),
                                "apply failed",
                            );
                        }
                    }
                }
                completed_rows = base.saturating_add(outcome.changes.len() as u64);
                if !rolled_back && outcome.changes.len() == rows.len() {
                    completed_rows = base.saturating_add(rows.len() as u64);
                }
                control.report(completed_rows, total_rows);
                batch_number = batch_number.saturating_add(1);
                !control.cancel_requested()
                    && !unknown
                    && failed_rows == 0
                    && conflict_rows == 0
                    && outcome.changes.len() == rows.len()
            },
        )
        .map_err(|_| ())
    });
    let _ = std::fs::remove_file(&frozen_path);
    if scan.is_err() && !control.cancel_requested() && !fatal && !unknown {
        fatal = true;
    }
    let cancelled = control.cancel_requested() && completed_rows < total_rows;
    let phase = if unknown {
        if cancelled {
            "unknown_after_cancel"
        } else {
            "unknown"
        }
    } else if cancelled && applied_rows > 0 {
        "cancelled_partial"
    } else if cancelled {
        "cancelled"
    } else if fatal || failed_rows > 0 || conflict_rows > 0 || completed_rows < total_rows {
        if applied_rows > 0 {
            "partial"
        } else {
            "failed"
        }
    } else {
        "completed"
    };
    if fatal {
        failed_rows = failed_rows.saturating_add(1);
        push_import_error(
            &mut errors,
            &mut errors_truncated,
            completed_rows.saturating_add(2),
            "stream validation or batch construction failed",
        );
    }
    Ok(StreamedCsvApplyOutcome {
        phase: phase.into(),
        completed_rows,
        applied_rows,
        conflict_rows,
        failed_rows,
        errors,
        errors_truncated,
        summary: format!(
            "{} · {} of {} processed · {} applied · {} conflict · {} failed",
            phase, completed_rows, total_rows, applied_rows, conflict_rows, failed_rows
        ),
    })
}

trait ImportDerivedId: Sized {
    fn from_parts(parts: IdParts) -> Result<Self, ()>;
}

impl ImportDerivedId for MutationId {
    fn from_parts(parts: IdParts) -> Result<Self, ()> {
        Self::from_parts(parts).map_err(|_| ())
    }
}

impl ImportDerivedId for ReviewTokenId {
    fn from_parts(parts: IdParts) -> Result<Self, ()> {
        Self::from_parts(parts).map_err(|_| ())
    }
}

fn derived_import_id<T: ImportDerivedId>(
    operation_id: OperationId,
    batch: u64,
    salt: u64,
) -> Result<T, ()> {
    let source = operation_id.parts();
    let high = source.high ^ 0x696d_706f_7274_0000 ^ salt;
    let mut low = source.low.wrapping_add(batch).wrapping_add(salt);
    if high == 0 && low == 0 {
        low = salt.max(1);
    }
    T::from_parts(IdParts::new(high, low).map_err(|_| ())?)
}

fn push_import_error(errors: &mut Vec<String>, truncated: &mut bool, row: u64, message: &str) {
    if errors.len() < 100 {
        errors.push(format!("row {row}: {message}"));
    } else {
        *truncated = true;
    }
}

fn parse_csv_value_types(
    mapped_types: &[String],
    column_count: usize,
) -> Result<Vec<CsvValueType>, BridgeError> {
    if mapped_types.len() != column_count {
        return Err(BridgeError::rejected(
            "csv-import-mapping",
            "mapped value types must match CSV width",
        ));
    }
    mapped_types
        .iter()
        .map(|value_type| match value_type.as_str() {
            "text" => Ok(CsvValueType::Text),
            "signed" => Ok(CsvValueType::Signed),
            "float64" => Ok(CsvValueType::Float64),
            "boolean" => Ok(CsvValueType::Boolean),
            _ => Err(BridgeError::rejected(
                "csv-import-mapping",
                "mapped value type must be text, signed, float64, or boolean",
            )),
        })
        .collect()
}

fn freeze_csv_source(path: &Path, token_id: ReviewTokenId) -> Result<PathBuf, BridgeError> {
    let metadata = path
        .metadata()
        .map_err(|error| BridgeError::rejected("csv-import-freeze", error.to_string()))?;
    if !metadata.is_file() {
        return Err(BridgeError::rejected(
            "csv-import-freeze",
            "CSV import source must be a regular file",
        ));
    }
    if metadata.len() > CSV_IMPORT_MAX_FILE_BYTES {
        return Err(BridgeError::rejected(
            "csv-import-size",
            format!(
                "CSV import has {} bytes; limit is {}",
                metadata.len(),
                CSV_IMPORT_MAX_FILE_BYTES
            ),
        ));
    }
    let frozen_path = std::env::temp_dir().join(format!(
        "tablerock-import-{}-{token_id}-{}.csv",
        std::process::id(),
        CSV_IMPORT_SPOOL_NONCE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut destination = options
        .open(&frozen_path)
        .map_err(|error| BridgeError::rejected("csv-import-freeze", error.to_string()))?;
    let result = (|| -> Result<(), BridgeError> {
        let mut source = File::open(path)
            .map_err(|error| BridgeError::rejected("csv-import-freeze", error.to_string()))?;
        let mut buffer = [0_u8; 64 * 1024];
        let mut copied = 0_u64;
        loop {
            let read = source
                .read(&mut buffer)
                .map_err(|error| BridgeError::rejected("csv-import-freeze", error.to_string()))?;
            if read == 0 {
                break;
            }
            copied = copied.saturating_add(read as u64);
            if copied > CSV_IMPORT_MAX_FILE_BYTES {
                return Err(BridgeError::rejected(
                    "csv-import-size",
                    "CSV import grew beyond the maximum while being frozen",
                ));
            }
            destination
                .write_all(&buffer[..read])
                .map_err(|error| BridgeError::rejected("csv-import-freeze", error.to_string()))?;
        }
        destination
            .sync_all()
            .map_err(|error| BridgeError::rejected("csv-import-freeze", error.to_string()))
    })();
    if let Err(error) = result {
        drop(destination);
        let _ = std::fs::remove_file(&frozen_path);
        return Err(error);
    }
    Ok(frozen_path)
}

fn expire_csv_import_reviews(
    reviews: &mut BTreeMap<ReviewTokenId, CsvImportReviewEntry>,
    now_ms: u64,
) {
    let expired = reviews
        .iter()
        .filter_map(|(token, review)| (review.expires_at_ms < now_ms).then_some(*token))
        .collect::<Vec<_>>();
    for token in expired {
        if let Some(review) = reviews.remove(&token) {
            let _ = std::fs::remove_file(&review.frozen_path);
        }
    }
}

fn preview_csv_import_inner(path: String) -> Result<BridgeCsvImportPreview, BridgeError> {
    let path_ref = Path::new(&path);
    if !path_ref.is_absolute() {
        return Err(BridgeError::rejected(
            "csv-import-path",
            "native CSV import path must be absolute",
        ));
    }
    let limits = csv_stream_limits()?;
    let mut headers = Vec::new();
    let mut rows = Vec::with_capacity(CSV_IMPORT_PREVIEW_ROWS);
    let summary = stream_csv_batches(path_ref, limits, |batch_headers, batch, _| {
        if headers.is_empty() {
            headers.extend_from_slice(batch_headers);
        }
        let remaining = CSV_IMPORT_PREVIEW_ROWS.saturating_sub(rows.len());
        rows.extend(batch.iter().take(remaining).cloned());
        true
    })
    .map_err(|error| BridgeError::rejected("csv-import", error.to_string()))?;
    Ok(BridgeCsvImportPreview {
        path,
        headers,
        rows: rows
            .into_iter()
            .map(|cells| BridgeCsvRow { cells })
            .collect(),
        total_rows: u32::try_from(summary.rows).unwrap_or(u32::MAX),
        formula_like_cells: u32::try_from(summary.formula_like_cells).unwrap_or(u32::MAX),
        fingerprint: summary
            .sha256
            .map(hex_sha256)
            .ok_or_else(|| BridgeError::rejected("csv-import", "CSV hash is unavailable"))?,
    })
}

fn hex_sha256(bytes: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(64);
    for byte in bytes {
        value.push(char::from(HEX[(byte >> 4) as usize]));
        value.push(char::from(HEX[(byte & 0x0f) as usize]));
    }
    value
}

fn csv_stream_limits() -> Result<CsvStreamLimits, BridgeError> {
    CsvStreamLimits::new(
        CSV_IMPORT_MAX_FILE_BYTES,
        CSV_IMPORT_MAX_ROWS,
        CSV_IMPORT_MAX_CELL_BYTES,
        CSV_IMPORT_BATCH_ROWS,
        8 * 1024 * 1024,
    )
    .map_err(|error| BridgeError::rejected("csv-import-limits", error.to_string()))
}

fn read_bridge_sql_file(raw_path: &str) -> Result<BridgeSqlFile, BridgeError> {
    let path = bridge_sql_path(raw_path)?;
    let (text, facts) = read_sql_file(&path)
        .map_err(|error| BridgeError::rejected("sql-file-read", error.to_string()))?;
    if text.len() > 8 * 1_048_576 {
        return Err(BridgeError::rejected(
            "sql-file-size",
            "SQL file exceeds 8 MiB",
        ));
    }
    Ok(bridge_sql_file(text, facts))
}

fn write_bridge_sql_file(
    raw_path: &str,
    statement_text: &str,
    expected_modified_nanos: Option<u64>,
    expected_len: Option<u64>,
    overwrite_external_change: bool,
) -> Result<BridgeSqlFile, BridgeError> {
    let path = bridge_sql_path(raw_path)?;
    if statement_text.len() > 8 * 1_048_576 {
        return Err(BridgeError::rejected(
            "sql-file-size",
            "SQL file exceeds 8 MiB",
        ));
    }
    if let (Some(modified_nanos), Some(len)) = (expected_modified_nanos, expected_len) {
        let previous = SqlFileFacts {
            path: path.clone(),
            mtime: Some(UNIX_EPOCH + Duration::from_nanos(modified_nanos)),
            len,
        };
        if !overwrite_external_change && external_change_detected(&previous) {
            return Err(BridgeError::rejected(
                "sql-file-external-change",
                "SQL file changed outside TableRock",
            ));
        }
    }
    let facts = write_sql_file_atomic(&path, statement_text)
        .map_err(|error| BridgeError::rejected("sql-file-write", error.to_string()))?;
    Ok(bridge_sql_file(statement_text.to_owned(), facts))
}

fn validate_session_intent(intent: &BridgeSessionIntent) -> Result<(), BridgeError> {
    if intent.database.len() > 256
        || intent
            .schema
            .as_ref()
            .is_some_and(|value| value.len() > 256)
        || intent.tabs.is_empty()
        || intent.tabs.len() > 64
        || usize::try_from(intent.selected_tab).map_or(true, |index| index >= intent.tabs.len())
        || intent.tabs.iter().any(|tab| {
            tab.title.trim().is_empty()
                || tab.title.len() > 256
                || tab.statement_text.len() > 1_048_576
        })
    {
        return Err(BridgeError::rejected(
            "session-intent",
            "session intent exceeds tab, selection, title, or text bounds",
        ));
    }
    Ok(())
}

fn encode_session_intent(intent: BridgeSessionIntent) -> Result<String, BridgeError> {
    serde_json::to_string(&serde_json::json!({
        "database": intent.database,
        "schema": intent.schema,
        "selected_tab": intent.selected_tab,
        "tabs": intent.tabs.into_iter().map(|tab| serde_json::json!({
            "title": tab.title,
            "sql": tab.statement_text,
        })).collect::<Vec<_>>(),
    }))
    .map_err(|_| BridgeError::rejected("session-intent", "cannot encode session intent"))
}

fn decode_session_intent(raw: &str) -> Result<BridgeSessionIntent, BridgeError> {
    let value: serde_json::Value = serde_json::from_str(raw)
        .map_err(|_| BridgeError::rejected("session-intent", "invalid session intent JSON"))?;
    let object = value.as_object().ok_or_else(|| {
        BridgeError::rejected("session-intent", "session intent must be an object")
    })?;
    let database = object
        .get("database")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let schema = match object.get("schema") {
        None | Some(serde_json::Value::Null) => None,
        Some(value) => Some(
            value
                .as_str()
                .ok_or_else(|| {
                    BridgeError::rejected("session-intent", "session schema must be text or null")
                })?
                .to_owned(),
        ),
    };
    let selected_tab = object
        .get("selected_tab")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| BridgeError::rejected("session-intent", "invalid selected tab"))?;
    let tabs = object
        .get("tabs")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| BridgeError::rejected("session-intent", "session tabs must be an array"))?
        .iter()
        .map(|value| {
            let tab = value.as_object().ok_or_else(|| {
                BridgeError::rejected("session-intent", "session tab must be an object")
            })?;
            Ok(BridgeWorkspaceTab {
                title: tab
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| {
                        BridgeError::rejected("session-intent", "session tab title missing")
                    })?
                    .to_owned(),
                statement_text: tab
                    .get("sql")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            })
        })
        .collect::<Result<Vec<_>, BridgeError>>()?;
    let intent = BridgeSessionIntent {
        database,
        schema,
        selected_tab,
        tabs,
    };
    validate_session_intent(&intent)?;
    Ok(intent)
}

#[derive(Clone, Copy)]
enum CatalogExpectedLevel {
    PostgreSqlDatabase,
    PostgreSqlSchema,
    PostgreSqlObject,
    ClickHouseDatabase,
    ClickHouseObject,
    RedisLogicalDatabase,
    RedisKey,
}

impl CatalogExpectedLevel {
    const fn accepts(self, kind: CatalogNodeKind) -> bool {
        matches!(
            (self, kind),
            (
                Self::PostgreSqlDatabase,
                CatalogNodeKind::PostgreSqlDatabase
            ) | (Self::PostgreSqlSchema, CatalogNodeKind::PostgreSqlSchema)
                | (Self::PostgreSqlObject, CatalogNodeKind::PostgreSqlObject(_))
                | (
                    Self::ClickHouseDatabase,
                    CatalogNodeKind::ClickHouseDatabase
                )
                | (Self::ClickHouseObject, CatalogNodeKind::ClickHouseObject(_))
                | (
                    Self::RedisLogicalDatabase,
                    CatalogNodeKind::RedisLogicalDatabase
                )
                | (Self::RedisKey, CatalogNodeKind::RedisKey(_))
        )
    }
}

fn bridge_catalog_node(node: &CatalogNode) -> BridgeCatalogNode {
    BridgeCatalogNode {
        id_bytes: catalog_node_bytes(node.id()),
        parent_id_bytes: node.parent_id().map(catalog_node_bytes),
        depth: node.depth(),
        name: redis_catalog_display_name(node),
        kind: catalog_kind_label(node.kind()).to_owned(),
        children_state: catalog_children_label(node.children()).to_owned(),
        expandable: catalog_kind_is_expandable(node.kind()),
    }
}

fn validate_bridge_saved_filter_preset(
    preset: &BridgeSavedFilterPreset,
) -> Result<(), BridgeError> {
    if !is_safe_preset_name(&preset.name) {
        return Err(BridgeError::rejected(
            "saved-filter-name",
            "preset name must be 1 to 64 ASCII letters, digits, dots, dashes, or underscores",
        ));
    }
    if preset.filters.len() > 32 {
        return Err(BridgeError::rejected(
            "saved-filter-bounds",
            "at most 32 filters are allowed",
        ));
    }
    for filter in &preset.filters {
        if filter.column.is_empty() || filter.column.len() > MAX_BROWSE_IDENTIFIER_BYTES {
            return Err(BridgeError::rejected(
                "saved-filter-bounds",
                "filter column must be 1 to 1024 bytes",
            ));
        }
        if filter
            .value
            .as_ref()
            .is_some_and(|value| value.len() > MAX_BROWSE_VALUE_BYTES)
        {
            return Err(BridgeError::rejected(
                "saved-filter-bounds",
                "filter value must be at most 65536 bytes",
            ));
        }
        let nullary = matches!(filter.operator.as_str(), "is_null" | "is_not_null");
        let valued = matches!(
            filter.operator.as_str(),
            "eq" | "ne" | "lt" | "le" | "gt" | "ge" | "like" | "ilike" | "not_like" | "not_ilike"
        );
        if (!nullary && !valued)
            || (nullary && filter.value.is_some())
            || (valued && filter.value.is_none())
        {
            return Err(BridgeError::rejected(
                "saved-filter-operator",
                "filter operator or value shape is invalid",
            ));
        }
    }
    if preset.raw_where.as_ref().is_some_and(|fragment| {
        fragment.trim().is_empty() || fragment.len() > MAX_BROWSE_VALUE_BYTES
    }) {
        return Err(BridgeError::rejected(
            "saved-filter-bounds",
            "raw WHERE must be non-empty and at most 65536 bytes",
        ));
    }
    Ok(())
}

fn bridge_saved_filter_preset(preset: SavedFilterPreset) -> BridgeSavedFilterPreset {
    BridgeSavedFilterPreset {
        name: preset.name,
        filters: preset
            .filters
            .into_iter()
            .map(|filter| BridgeBrowseFilter {
                column: filter.column,
                operator: filter.operator,
                value: filter.value,
            })
            .collect(),
        raw_where: preset.raw_where,
    }
}

fn redis_catalog_display_name(node: &CatalogNode) -> String {
    if !matches!(node.kind(), CatalogNodeKind::RedisKey(_)) {
        return node.name().to_owned();
    }
    node.name()
        .strip_prefix("text:")
        .map(str::to_owned)
        .or_else(|| {
            node.name()
                .strip_prefix("hex:")
                .map(|hex| format!("[binary] {hex}"))
        })
        .unwrap_or_else(|| node.name().to_owned())
}

fn decode_redis_catalog_key(identity: &str) -> Result<Vec<u8>, BridgeError> {
    if let Some(text) = identity.strip_prefix("text:") {
        if text.len() > 8 * 1024 {
            return Err(BridgeError::rejected(
                "redis-key-identity",
                "Redis key exceeds view limit",
            ));
        }
        return Ok(text.as_bytes().to_vec());
    }
    let hex = identity.strip_prefix("hex:").ok_or_else(|| {
        BridgeError::rejected("redis-key-identity", "Redis key identity is invalid")
    })?;
    if hex.len() > 16 * 1024 || hex.len() % 2 != 0 {
        return Err(BridgeError::rejected(
            "redis-key-identity",
            "Redis key hex identity is invalid",
        ));
    }
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).map_err(|_| {
                BridgeError::rejected("redis-key-identity", "Redis key hex identity is invalid")
            })?;
            u8::from_str_radix(text, 16).map_err(|_| {
                BridgeError::rejected("redis-key-identity", "Redis key hex identity is invalid")
            })
        })
        .collect()
}

const fn catalog_kind_is_expandable(kind: CatalogNodeKind) -> bool {
    matches!(
        kind,
        CatalogNodeKind::PostgreSqlDatabase
            | CatalogNodeKind::PostgreSqlSchema
            | CatalogNodeKind::ClickHouseDatabase
            | CatalogNodeKind::RedisLogicalDatabase
    )
}

const fn catalog_children_label(state: CatalogChildrenState) -> &'static str {
    match state {
        CatalogChildrenState::NotApplicable => "not_applicable",
        CatalogChildrenState::Unrequested => "unrequested",
        CatalogChildrenState::Loading => "loading",
        CatalogChildrenState::Loaded { complete: true } => "loaded_complete",
        CatalogChildrenState::Loaded { complete: false } => "loaded_partial",
        CatalogChildrenState::Stale => "stale",
        CatalogChildrenState::Failed => "failed",
    }
}

fn role_change_summary(kind: &RoleChangeKind) -> String {
    match kind {
        RoleChangeKind::GrantMembership { role, member } => {
            format!("Grant role {role} to {member}")
        }
        RoleChangeKind::RevokeMembership { role, member } => {
            format!("Revoke role {role} from {member}")
        }
        RoleChangeKind::GrantTablePrivilege {
            schema,
            table,
            grantee,
            privilege,
        } => {
            format!("Grant {privilege} on {schema}.{table} to {grantee}")
        }
        RoleChangeKind::RevokeTablePrivilege {
            schema,
            table,
            grantee,
            privilege,
        } => {
            format!("Revoke {privilege} on {schema}.{table} from {grantee}")
        }
    }
}

fn preview_table_operation(plan: &DdlPlan) -> Result<String, BridgeError> {
    let quote = |identifier: &str| {
        tablerock_engine::quote_ident(identifier)
            .map_err(|error| BridgeError::rejected("table-operation-identifier", error.to_string()))
    };
    let statement = match (&plan.kind, &plan.target) {
        (DdlKind::RenameTable, DdlTarget::PostgreSqlRelation { schema, relation }) => {
            let new_name = plan.object_name.as_deref().ok_or_else(|| {
                BridgeError::rejected("table-operation-name", "new table name required")
            })?;
            format!(
                "ALTER TABLE {}.{} RENAME TO {}",
                quote(schema)?,
                quote(relation)?,
                quote(new_name)?
            )
        }
        (DdlKind::TruncateTable, DdlTarget::PostgreSqlRelation { schema, relation }) => {
            format!("TRUNCATE TABLE {}.{}", quote(schema)?, quote(relation)?)
        }
        (DdlKind::DropTable, DdlTarget::PostgreSqlRelation { schema, relation }) => {
            format!("DROP TABLE {}.{}", quote(schema)?, quote(relation)?)
        }
        (DdlKind::Vacuum, DdlTarget::PostgreSqlRelation { schema, relation }) => {
            format!("VACUUM {}.{}", quote(schema)?, quote(relation)?)
        }
        (DdlKind::Analyze, DdlTarget::PostgreSqlRelation { schema, relation }) => {
            format!("ANALYZE {}.{}", quote(schema)?, quote(relation)?)
        }
        (DdlKind::Optimize, DdlTarget::ClickHouseTable { database, table }) => {
            format!("OPTIMIZE TABLE {}.{}", quote(database)?, quote(table)?)
        }
        _ => {
            return Err(BridgeError::rejected(
                "table-operation-plan",
                "plan is not a supported table operation",
            ));
        }
    };
    Ok(format!("{statement};"))
}

fn preview_postgres_ddl(plan: &DdlPlan) -> Result<String, BridgeError> {
    let DdlTarget::PostgreSqlRelation { schema, relation } = &plan.target else {
        return Err(BridgeError::rejected(
            "ddl-change-target",
            "PostgreSQL relation target required",
        ));
    };
    let quote = |identifier: &str| {
        tablerock_engine::quote_ident(identifier)
            .map_err(|error| BridgeError::rejected("ddl-change-identifier", error.to_string()))
    };
    let qualified = format!("{}.{}", quote(schema)?, quote(relation)?);
    let statement =
        match plan.kind {
            DdlKind::AddColumn => {
                let name = plan.object_name.as_deref().ok_or_else(|| {
                    BridgeError::rejected("ddl-change-object", "column name required")
                })?;
                let definition = plan.type_text.as_deref().ok_or_else(|| {
                    BridgeError::rejected("ddl-change-definition", "column type required")
                })?;
                if !definition.chars().all(|character| {
                    character.is_ascii_alphanumeric()
                        || matches!(character, '(' | ')' | ',' | ' ' | '"')
                }) {
                    return Err(BridgeError::rejected(
                        "ddl-change-definition",
                        "column type contains unsupported syntax",
                    ));
                }
                format!(
                    "ALTER TABLE {qualified} ADD COLUMN {} {definition}",
                    quote(name)?
                )
            }
            DdlKind::DropColumn => {
                let name = plan.object_name.as_deref().ok_or_else(|| {
                    BridgeError::rejected("ddl-change-object", "column name required")
                })?;
                format!("ALTER TABLE {qualified} DROP COLUMN {}", quote(name)?)
            }
            DdlKind::CreateIndex => {
                let name = plan.object_name.as_deref().ok_or_else(|| {
                    BridgeError::rejected("ddl-change-object", "index name required")
                })?;
                let columns = plan.type_text.as_deref().ok_or_else(|| {
                    BridgeError::rejected("ddl-change-definition", "index columns required")
                })?;
                if !columns.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '_' | ',' | ' ' | '"')
                }) {
                    return Err(BridgeError::rejected(
                        "ddl-change-definition",
                        "index columns contain unsupported syntax",
                    ));
                }
                let columns = columns
                    .split(',')
                    .map(str::trim)
                    .filter(|column| !column.is_empty())
                    .map(quote)
                    .collect::<Result<Vec<_>, _>>()?;
                if columns.is_empty() {
                    return Err(BridgeError::rejected(
                        "ddl-change-definition",
                        "at least one index column is required",
                    ));
                }
                format!(
                    "CREATE INDEX {} ON {qualified} ({})",
                    quote(name)?,
                    columns.join(", ")
                )
            }
            DdlKind::DropIndex => {
                let name = plan.object_name.as_deref().ok_or_else(|| {
                    BridgeError::rejected("ddl-change-object", "index name required")
                })?;
                format!("DROP INDEX {}.{}", quote(schema)?, quote(name)?)
            }
            DdlKind::AddConstraint => {
                let name = plan.object_name.as_deref().ok_or_else(|| {
                    BridgeError::rejected("ddl-change-object", "constraint name required")
                })?;
                let definition = plan.type_text.as_deref().ok_or_else(|| {
                    BridgeError::rejected("ddl-change-definition", "constraint definition required")
                })?;
                let upper = definition.trim().to_ascii_uppercase();
                if !(upper.starts_with("UNIQUE")
                    || upper.starts_with("PRIMARY KEY")
                    || upper.starts_with("CHECK"))
                    || !definition.chars().all(|character| {
                        character.is_ascii_alphanumeric() || " _(),.><=!\"'+-*/".contains(character)
                    })
                {
                    return Err(BridgeError::rejected(
                        "ddl-change-definition",
                        "constraint must be UNIQUE, PRIMARY KEY, or CHECK with bounded syntax",
                    ));
                }
                format!(
                    "ALTER TABLE {qualified} ADD CONSTRAINT {} {}",
                    quote(name)?,
                    definition.trim()
                )
            }
            DdlKind::DropConstraint => {
                let name = plan.object_name.as_deref().ok_or_else(|| {
                    BridgeError::rejected("ddl-change-object", "constraint name required")
                })?;
                format!("ALTER TABLE {qualified} DROP CONSTRAINT {}", quote(name)?)
            }
            _ => {
                return Err(BridgeError::rejected(
                    "ddl-change-kind",
                    "operation is not a structure change",
                ));
            }
        };
    Ok(format!("{statement};"))
}

const fn catalog_kind_label(kind: CatalogNodeKind) -> &'static str {
    match kind {
        CatalogNodeKind::PostgreSqlDatabase => "postgresql_database",
        CatalogNodeKind::PostgreSqlSchema => "postgresql_schema",
        CatalogNodeKind::PostgreSqlObject(PostgreSqlObjectKind::Table) => "postgresql_table",
        CatalogNodeKind::PostgreSqlObject(PostgreSqlObjectKind::View) => "postgresql_view",
        CatalogNodeKind::PostgreSqlObject(PostgreSqlObjectKind::MaterializedView) => {
            "postgresql_materialized_view"
        }
        CatalogNodeKind::PostgreSqlObject(PostgreSqlObjectKind::ForeignTable) => {
            "postgresql_foreign_table"
        }
        CatalogNodeKind::PostgreSqlObject(PostgreSqlObjectKind::PartitionedTable) => {
            "postgresql_partitioned_table"
        }
        CatalogNodeKind::PostgreSqlObject(PostgreSqlObjectKind::Sequence) => "postgresql_sequence",
        CatalogNodeKind::PostgreSqlObject(PostgreSqlObjectKind::Function) => "postgresql_function",
        CatalogNodeKind::PostgreSqlObject(PostgreSqlObjectKind::Type) => "postgresql_type",
        CatalogNodeKind::PostgreSqlColumn => "postgresql_column",
        CatalogNodeKind::ClickHouseDatabase => "clickhouse_database",
        CatalogNodeKind::ClickHouseObject(ClickHouseObjectKind::Table) => "clickhouse_table",
        CatalogNodeKind::ClickHouseObject(ClickHouseObjectKind::View) => "clickhouse_view",
        CatalogNodeKind::ClickHouseObject(ClickHouseObjectKind::MaterializedView) => {
            "clickhouse_materialized_view"
        }
        CatalogNodeKind::ClickHouseObject(ClickHouseObjectKind::Dictionary) => {
            "clickhouse_dictionary"
        }
        CatalogNodeKind::ClickHouseColumn => "clickhouse_column",
        CatalogNodeKind::RedisLogicalDatabase => "redis_logical_database",
        CatalogNodeKind::RedisNamespace => "redis_namespace",
        CatalogNodeKind::RedisKey(RedisKeyKind::Unknown) => "redis_key_unknown",
        CatalogNodeKind::RedisKey(RedisKeyKind::String) => "redis_key_string",
        CatalogNodeKind::RedisKey(RedisKeyKind::Hash) => "redis_key_hash",
        CatalogNodeKind::RedisKey(RedisKeyKind::List) => "redis_key_list",
        CatalogNodeKind::RedisKey(RedisKeyKind::Set) => "redis_key_set",
        CatalogNodeKind::RedisKey(RedisKeyKind::SortedSet) => "redis_key_sorted_set",
        CatalogNodeKind::RedisKey(RedisKeyKind::Stream) => "redis_key_stream",
    }
}

impl BridgeInner {
    fn new() -> Result<Self, BridgeError> {
        let core = ServiceCoordinator::new(
            ServiceLimits::new(256, 256, 8, 64)
                .map_err(|error| BridgeError::rejected("service-limits", error.to_string()))?,
        );
        let runtime = DriverRuntime::new(64, 32)
            .map_err(|error| BridgeError::rejected("driver-runtime", format!("{error:?}")))?;
        let service = EngineService::new(core, runtime, MAX_SESSIONS)
            .map_err(|error| BridgeError::rejected("engine-service", error.to_string()))?;
        let results = ResultStore::new(
            ResultStoreLimits::new(32, 64, 64 * 2 * 1024 * 1024)
                .map_err(|error| BridgeError::rejected("result-store", error.to_string()))?,
        );
        let reviews = MutationReviewRegistry::new(256)
            .map_err(|error| BridgeError::rejected("review-registry", error.to_string()))?;
        Ok(Self {
            service,
            results,
            reviews,
            csv_import_reviews: BTreeMap::new(),
            role_reviews: BTreeMap::new(),
            ddl_reviews: BTreeMap::new(),
            sessions: BTreeMap::new(),
            operation_results: BTreeMap::new(),
            operation_history: BTreeMap::new(),
            operation_copy_identity: BTreeMap::new(),
            history_retention: HistoryRetention::Full,
            ids: IdFactory::new(),
            events: VecDeque::new(),
            next_sequence: 0,
            first_sequence: 0,
            accepting: true,
            persistence: None,
            catalog_nodes: BTreeMap::new(),
            copy_identities: BTreeMap::new(),
            support_bundle: SupportBundle::new(SupportPlatform::current()),
        })
    }

    fn push_event(&mut self, mut event: BridgeEventRecord) {
        event.sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.events.push_back(event);
        while self.events.len() > MAX_EVENT_LOG {
            self.events.pop_front();
            self.first_sequence = self.first_sequence.saturating_add(1);
        }
    }

    fn apply_update(
        &mut self,
        operation_id: OperationId,
        update: EngineServiceUpdate,
    ) -> Result<bool, BridgeError> {
        let op_bytes = operation_bytes(operation_id);
        match update {
            EngineServiceUpdate::Started => {
                self.push_event(BridgeEventRecord {
                    sequence: 0,
                    operation_id: op_bytes,
                    kind: "started".into(),
                    outcome: None,
                    rows: None,
                    bytes: None,
                    page_bytes: None,
                });
                Ok(false)
            }
            EngineServiceUpdate::Page(page) => {
                let page = *page;
                if let Some(copy_identity) =
                    self.operation_copy_identity.get(&operation_id).cloned()
                {
                    while self.copy_identities.len() >= 32 {
                        let Some(oldest) = self.copy_identities.keys().next().copied() else {
                            break;
                        };
                        self.copy_identities.remove(&oldest);
                    }
                    self.copy_identities
                        .insert(page.envelope().result_id(), copy_identity);
                }
                let identity = PageIdentity::new(
                    page.envelope().result_id(),
                    page.envelope().revision(),
                    page.envelope().engine(),
                );
                self.results
                    .open_result(identity)
                    .map_err(|error| BridgeError::rejected("open-result", error.to_string()))?;
                self.results
                    .admit(page.clone())
                    .map_err(|error| BridgeError::rejected("admit", error.to_string()))?;
                self.push_event(BridgeEventRecord {
                    sequence: 0,
                    operation_id: op_bytes,
                    kind: "page".into(),
                    outcome: None,
                    rows: Some(u64::from(page.envelope().row_count())),
                    bytes: Some(page.envelope().arena_byte_len()),
                    page_bytes: Some(page.encode_v1()),
                });
                Ok(false)
            }
            EngineServiceUpdate::CancelDispatched(dispatch) => {
                self.push_event(BridgeEventRecord {
                    sequence: 0,
                    operation_id: op_bytes,
                    kind: "cancel_dispatched".into(),
                    outcome: Some(format!("{dispatch:?}")),
                    rows: None,
                    bytes: None,
                    page_bytes: None,
                });
                Ok(false)
            }
            EngineServiceUpdate::Terminal(outcome) => {
                if let Some(diagnostic) = self.service.take_terminal_diagnostic(operation_id) {
                    let _ = self.support_bundle.push(&diagnostic);
                }
                if !matches!(outcome, OperationOutcome::Completed)
                    && let Some(identity) = self.operation_results.get(&operation_id)
                {
                    let _ = self
                        .support_bundle
                        .push_operation_outcome(identity.engine(), outcome);
                }
                self.operation_results.remove(&operation_id);
                self.operation_copy_identity.remove(&operation_id);
                let history_failed =
                    self.operation_history
                        .remove(&operation_id)
                        .is_some_and(|mut history| {
                            history.outcome = history_outcome(outcome);
                            self.persistence
                                .as_ref()
                                .is_some_and(|actor| actor.append_history(history).is_err())
                        });
                if history_failed {
                    self.push_event(BridgeEventRecord {
                        sequence: 0,
                        operation_id: op_bytes.clone(),
                        kind: "history_failed".into(),
                        outcome: None,
                        rows: None,
                        bytes: None,
                        page_bytes: None,
                    });
                }
                self.push_event(BridgeEventRecord {
                    sequence: 0,
                    operation_id: op_bytes,
                    kind: "terminal".into(),
                    outcome: Some(outcome_label(outcome).into()),
                    rows: None,
                    bytes: None,
                    page_bytes: None,
                });
                Ok(true)
            }
        }
    }
}

const fn history_outcome(outcome: OperationOutcome) -> HistoryOutcomeClass {
    match outcome {
        OperationOutcome::Completed | OperationOutcome::CompletedBeforeCancel => {
            HistoryOutcomeClass::Completed
        }
        OperationOutcome::ClientStopped | OperationOutcome::ServerConfirmedCancelled => {
            HistoryOutcomeClass::Cancelled
        }
        OperationOutcome::Failed => HistoryOutcomeClass::Failed,
        OperationOutcome::Disconnected => HistoryOutcomeClass::Disconnected,
        OperationOutcome::Unknown => HistoryOutcomeClass::Unknown,
    }
}

fn outcome_label(outcome: OperationOutcome) -> &'static str {
    match outcome {
        OperationOutcome::Completed => "completed",
        OperationOutcome::CompletedBeforeCancel => "completed_before_cancel",
        OperationOutcome::ClientStopped => "client_stopped",
        OperationOutcome::ServerConfirmedCancelled => "server_confirmed_cancelled",
        OperationOutcome::Failed => "failed",
        OperationOutcome::Unknown => "unknown",
        OperationOutcome::Disconnected => "disconnected",
    }
}

const fn engine_label(engine: Engine) -> &'static str {
    match engine {
        Engine::PostgreSql => "postgresql",
        Engine::ClickHouse => "clickhouse",
        Engine::Redis => "redis",
    }
}

fn parse_engine(name: &str) -> Result<Engine, BridgeError> {
    match name.to_ascii_lowercase().as_str() {
        "postgresql" | "postgres" | "pg" => Ok(Engine::PostgreSql),
        "clickhouse" | "ch" => Ok(Engine::ClickHouse),
        "redis" => Ok(Engine::Redis),
        _ => Err(BridgeError::rejected(
            "engine",
            "engine must be postgresql, clickhouse, or redis",
        )),
    }
}

fn starts_with_explain_keyword(statement: &str) -> bool {
    let Some(rest) = statement.get("EXPLAIN".len()..) else {
        return false;
    };
    statement[.."EXPLAIN".len()].eq_ignore_ascii_case("EXPLAIN")
        && rest
            .chars()
            .next()
            .is_none_or(|next| next.is_ascii_whitespace() || next == '(')
}

fn parse_bridge_query_parameter(
    binding: BridgeQueryParameter,
) -> Result<(tablerock_engine::FilterValue, &'static str), BridgeError> {
    let missing = || {
        BridgeError::rejected(
            "named-parameter-value",
            "non-null parameter requires a value",
        )
    };
    match binding.kind.as_str() {
        "text" => Ok((
            tablerock_engine::FilterValue::Text(binding.value.ok_or_else(missing)?),
            "String",
        )),
        "integer" => {
            let raw = binding.value.ok_or_else(missing)?;
            let value = raw.parse::<i64>().map_err(|_| {
                BridgeError::rejected("named-parameter-integer", "invalid 64-bit integer")
            })?;
            Ok((tablerock_engine::FilterValue::Integer(value), "Int64"))
        }
        "float" => {
            let raw = binding.value.ok_or_else(missing)?;
            let value = raw.parse::<f64>().map_err(|_| {
                BridgeError::rejected("named-parameter-float", "invalid 64-bit float")
            })?;
            if !value.is_finite() {
                return Err(BridgeError::rejected(
                    "named-parameter-float",
                    "float must be finite",
                ));
            }
            Ok((tablerock_engine::FilterValue::Float(value), "Float64"))
        }
        "boolean" => {
            let raw = binding.value.ok_or_else(missing)?;
            let value = match raw.to_ascii_lowercase().as_str() {
                "true" => true,
                "false" => false,
                _ => {
                    return Err(BridgeError::rejected(
                        "named-parameter-boolean",
                        "boolean must be true or false",
                    ));
                }
            };
            Ok((tablerock_engine::FilterValue::Boolean(value), "Bool"))
        }
        "null" if binding.value.is_none() => {
            Ok((tablerock_engine::FilterValue::Null, "Nullable(String)"))
        }
        "null" => Err(BridgeError::rejected(
            "named-parameter-null",
            "null parameter cannot carry a value",
        )),
        _ => Err(BridgeError::rejected(
            "named-parameter-kind",
            "parameter kind must be text, integer, float, boolean, or null",
        )),
    }
}
