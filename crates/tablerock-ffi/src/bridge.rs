//! Coarse synchronous facade matching the shared-client-contract bridge shape.

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::{Arc, Mutex},
    time::Duration,
};

use tablerock_core::{
    BoundedText, ByteLimit, CatalogChildrenState, CatalogNode, CatalogNodeId, CatalogNodeKind,
    ClickHouseObjectKind, CommandBudget, CommandBudgetLimits, CommandEnvelope, CommandIntent,
    CommandScope, DangerousPlaintext, Engine, EnvironmentReference, EnvironmentTag, FieldValue,
    MutationChange, MutationPlan, MutationPlanLimits, MutationReviewRegistry, MutationTarget,
    OnePasswordReference, OperationId, OperationOutcome, OperationScope, OwnedValue, PageIdentity,
    PageKey, PageRequest, PlaintextAcknowledgement, PostgreSqlObjectKind, ProfileAggregate,
    ProfileConnectionSnapshot, ProfileDurability, ProfileEndpointPart, ProfileGroupName, ProfileId,
    ProfileIdentity, ProfileLimits, ProfileListFilter, ProfileListRequest, ProfileName,
    ProfileOrganization, ProfilePolicy, ProfilePreferences, ProfileProperty,
    ProfilePropertyBinding, ProfilePropertySet, ProfileSafetyMode, ProfileSearchTerm, ProfileTag,
    ReconnectDecision, ReconnectPreference, RedisKeyKind, ResultStore, ResultStoreLimits, Revision,
    SecretSource, SecretSourceKind, ServiceCoordinator, ServiceLimits, SessionId, ShutdownMode,
    StatementText, TlsPolicy, reconnect_decision,
};
use tablerock_engine::{
    AdapterFailureClass, CatalogRequest, ClickHouseCompression, ClickHouseConnectConfig,
    ClickHouseProbeQuery, ClickHouseSession, ClickHouseTlsMode, DriverPageRequest, DriverRuntime,
    DriverSession, EngineService, EngineServiceUpdate, PostgresConnectConfig, PostgresProbeQuery,
    PostgresSession, PostgresTlsMode, RedisConnectConfig, RedisConnectionSecurity,
    RedisCredentials, RedisProtocol, RedisSession, RedisTlsMode, ResolvedSecret, SecretPromptPort,
    SecretResolutionError, resolve_for_connect,
};
use tablerock_persistence::{PersistenceActor, ProfileOrderUpdate};

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

#[derive(Clone, Copy)]
struct RegisteredSession {
    profile_id: tablerock_core::ProfileId,
    session_id: SessionId,
    context_id: tablerock_core::ContextId,
    engine: Engine,
    /// Expected context revision tracked by the bridge.
    context_revision: Revision,
}

struct BridgeInner {
    service: EngineService,
    results: ResultStore,
    reviews: MutationReviewRegistry,
    sessions: BTreeMap<SessionId, RegisteredSession>,
    /// Operation -> result identity used when admitting streamed pages.
    operation_results: BTreeMap<OperationId, PageIdentity>,
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
}

#[uniffi::export]
impl TableRockBridge {
    #[uniffi::constructor]
    #[must_use]
    pub fn create() -> Arc<Self> {
        Arc::new(Self {
            runtime: RuntimeOwner::new(),
            inner: Mutex::new(None),
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
        catch_entry(|| self.open_profile_inner(profile_id, password_override))
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
        catch_entry(|| self.test_profile_inner(profile_id, password_override))
    }

    pub fn list_profile_groups(&self) -> Result<Vec<BridgeProfileGroup>, BridgeError> {
        catch_entry(|| self.list_profile_groups_inner())
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

    /// Graceful or cancel-active shutdown. `deadline_ms` reserved for future hard caps.
    pub fn shutdown(
        &self,
        cancel_active: bool,
        _deadline_ms: u64,
    ) -> Result<ShutdownOutcome, BridgeError> {
        catch_entry(|| self.shutdown_inner(cancel_active))
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
        catch_entry(|| self.reconnect_saved_session_inner(session_id, password_override))
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
        password_override: Option<String>,
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
        catch_entry(|| self.open_driver_session_inner(engine, session, None))
    }

    /// Registers a test driver under an existing saved-profile identity.
    /// Not exported to UniFFI.
    pub fn open_driver_session_for_profile(
        &self,
        profile_id: ProfileId,
        engine: Engine,
        session: Box<dyn DriverSession>,
    ) -> Result<Vec<u8>, BridgeError> {
        catch_entry(|| self.open_driver_session_inner(engine, session, Some(profile_id)))
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

    fn disconnect_inner(&self, session_id_bytes: Vec<u8>) -> Result<(), BridgeError> {
        self.ensure_runtime_inner()?;
        let session_id = session_from_bytes(&session_id_bytes)
            .map_err(|_| BridgeError::rejected("bad-session-id", "session id must be 16 bytes"))?;
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
        inner.sessions.remove(&session_id);
        Ok(())
    }

    fn configure_persistence_inner(&self, path: String) -> Result<(), BridgeError> {
        self.ensure_runtime_inner()?;
        let actor = PersistenceActor::open(&path)
            .map_err(|error| BridgeError::rejected("persistence-open", error.to_string()))?;
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
        if let Some(previous) = inner.persistence.take() {
            let _ = previous.shutdown();
        }
        inner.persistence = Some(actor);
        Ok(())
    }

    fn open_profile_inner(
        &self,
        profile_id_bytes: Vec<u8>,
        password_override: Option<String>,
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
            struct OverridePrompt(Option<String>);
            impl SecretPromptPort for OverridePrompt {
                fn request(
                    &mut self,
                    _field: tablerock_core::SecretField,
                    _profile: &ProfileName,
                ) -> Result<ResolvedSecret, SecretResolutionError> {
                    self.0
                        .take()
                        .map(|value| ResolvedSecret::from_prompt(value.into_bytes(), _field))
                        .transpose()?
                        .ok_or(SecretResolutionError::PromptFailed)
                }
            }
            let password = if let Some(binding) = props.binding(ProfileProperty::Password) {
                let mut prompt = OverridePrompt(password_override);
                resolve_for_connect(binding, connection.name(), &mut prompt)
                    .map_err(|error| BridgeError::rejected("profile-password", error.to_string()))?
                    .map(|secret| String::from_utf8_lossy(secret.as_bytes()).into_owned())
                    .unwrap_or_default()
            } else {
                password_override.ok_or_else(|| {
                    BridgeError::rejected(
                        "profile-password",
                        "prompt-on-connect profile requires a password override",
                    )
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
        password_override: Option<String>,
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
        let registered = *inner
            .sessions
            .get(&session_id)
            .ok_or(BridgeError::UnknownSession)?;
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
                || matches!(
                    seed.children(),
                    CatalogChildrenState::NotApplicable | CatalogChildrenState::Failed
                )
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
                            database,
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
                            database,
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

        self.open_driver_session_inner(engine, session, saved_profile_id)
    }

    fn open_driver_session_inner(
        &self,
        engine: Engine,
        session: Box<dyn DriverSession>,
        saved_profile_id: Option<ProfileId>,
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

        inner
            .service
            .core_mut()
            .register_scope(CommandScope::Profile(profile_id), Revision::INITIAL)
            .map_err(|error| BridgeError::rejected("register-profile", error.to_string()))?;
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
                context_revision: Revision::INITIAL,
            },
        );
        Ok(session_bytes(session_id))
    }

    fn submit_inner(&self, spec: SubmitSpec) -> Result<Vec<u8>, BridgeError> {
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

            let (intent, request) = match intent_name.as_str() {
                "probe" | "execute" => {
                    let statement = spec.statement.clone().unwrap_or_else(|| "select 1".into());
                    let text = StatementText::new(statement)
                        .map_err(|error| BridgeError::rejected("statement", error.to_string()))?;
                    let limits = default_page_limits();
                    let max_cell_bytes = 64 * 1024;
                    let request = match (engine, intent_name.as_str()) {
                        (Engine::PostgreSql, "execute") => DriverPageRequest::PostgreSqlStatement {
                            statement: text.clone(),
                            parameters: Vec::new(),
                            limits,
                            max_cell_bytes,
                        },
                        (Engine::PostgreSql, _) => DriverPageRequest::PostgreSqlProbe {
                            query: PostgresProbeQuery::BoundedSeries,
                            limits,
                            max_cell_bytes,
                        },
                        (Engine::ClickHouse, "execute") => DriverPageRequest::ClickHouseStatement {
                            statement: text.clone(),
                            query_id: BoundedText::copy_from_str(
                                &format!("bridge-{}", page_identity.result_id()),
                                ByteLimit::new(128),
                            )
                            .map_err(|error| {
                                BridgeError::rejected("query-id", error.to_string())
                            })?,
                            limits,
                            max_cell_bytes,
                        },
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

    fn shutdown_inner(&self, cancel_active: bool) -> Result<ShutdownOutcome, BridgeError> {
        self.ensure_runtime_inner()?;
        let mode = if cancel_active {
            ShutdownMode::CancelActive
        } else {
            ShutdownMode::Graceful
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
        // Drain active ops when possible.
        let active = match outcome.core {
            tablerock_core::ShutdownOutcome::Draining { active_operations } => active_operations,
            tablerock_core::ShutdownOutcome::Stopped
            | tablerock_core::ShutdownOutcome::AlreadyStopped => 0,
        };
        if active == 0 {
            let _ = self.runtime.block_on(inner.service.complete_shutdown());
            drop(guard);
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
            *guard = None;
            let _ = self.runtime.shutdown();
        }
        Ok(ShutdownOutcome {
            core: format!("{:?}", outcome.core),
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
            return Err(BridgeError::rejected(
                "profile-password",
                "Keychain editing is not available yet",
            ));
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

#[derive(Clone, Copy)]
enum CatalogExpectedLevel {
    PostgreSqlDatabase,
    PostgreSqlSchema,
    PostgreSqlObject,
    ClickHouseDatabase,
    ClickHouseObject,
    RedisLogicalDatabase,
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
        )
    }
}

fn bridge_catalog_node(node: &CatalogNode) -> BridgeCatalogNode {
    BridgeCatalogNode {
        id_bytes: catalog_node_bytes(node.id()),
        parent_id_bytes: node.parent_id().map(catalog_node_bytes),
        depth: node.depth(),
        name: node.name().to_owned(),
        kind: catalog_kind_label(node.kind()).to_owned(),
        children_state: catalog_children_label(node.children()).to_owned(),
        expandable: catalog_kind_is_expandable(node.kind()),
    }
}

const fn catalog_kind_is_expandable(kind: CatalogNodeKind) -> bool {
    matches!(
        kind,
        CatalogNodeKind::PostgreSqlDatabase
            | CatalogNodeKind::PostgreSqlSchema
            | CatalogNodeKind::ClickHouseDatabase
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
            sessions: BTreeMap::new(),
            operation_results: BTreeMap::new(),
            ids: IdFactory::new(),
            events: VecDeque::new(),
            next_sequence: 0,
            first_sequence: 0,
            accepting: true,
            persistence: None,
            catalog_nodes: BTreeMap::new(),
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
                self.operation_results.remove(&operation_id);
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
