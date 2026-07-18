//! Coarse synchronous facade matching the shared-client-contract bridge shape.

use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
};

use tablerock_core::{
    BoundedText, ByteLimit, CommandBudget, CommandBudgetLimits, CommandEnvelope, CommandIntent,
    CommandScope, Engine, FieldValue, MutationChange, MutationPlan, MutationPlanLimits,
    MutationReviewRegistry, MutationTarget, OperationId, OperationOutcome, OperationScope,
    OwnedValue, PageIdentity, PageKey, PageRequest, ProfileId, ProfileListFilter,
    ProfileListRequest, ProfileProperty, ResultStore, ResultStoreLimits, Revision,
    ServiceCoordinator, ServiceLimits, SessionId, ShutdownMode, StatementText,
};
use tablerock_engine::{
    ClickHouseCompression, ClickHouseConnectConfig, ClickHouseProbeQuery, ClickHouseSession,
    ClickHouseTlsMode, DriverPageRequest, DriverRuntime, DriverSession, EngineService,
    EngineServiceUpdate, PostgresConnectConfig, PostgresProbeQuery, PostgresSession,
    PostgresTlsMode, RedisConnectConfig, RedisConnectionSecurity, RedisCredentials, RedisProtocol,
    RedisSession, RedisTlsMode,
};
use tablerock_persistence::PersistenceActor;

use crate::{
    error::{BridgeError, catch_entry},
    ids::{
        IdFactory, operation_bytes, operation_from_bytes, result_from_bytes, review_token_bytes,
        review_token_from_bytes, session_bytes, session_from_bytes,
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
            .finish()
    }
}

/// Submits one coarse command. Intent-specific fields are optional by kind.
#[derive(Debug, Clone, uniffi::Record)]
pub struct SubmitSpec {
    /// `execute`, `fetch_page`, `refresh_catalog`, or `probe`.
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
}

/// One saved profile row for the native connection screen.
#[derive(Debug, Clone, uniffi::Record)]
pub struct BridgeProfileItem {
    /// 16-byte ProfileId (same form `open_profile` accepts).
    pub id_bytes: Vec<u8>,
    pub name: String,
    pub engine: String,
    pub group: Option<String>,
    pub favorite: bool,
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
        catch_entry(|| self.list_profiles_inner())
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
}

impl TableRockBridge {
    /// Registers an already-constructed driver session (unit/conformance tests).
    ///
    /// Not exported to UniFFI — Rust-only seam for in-process tests.
    pub fn open_driver_session(
        &self,
        engine: Engine,
        session: Box<dyn DriverSession>,
    ) -> Result<Vec<u8>, BridgeError> {
        catch_entry(|| self.open_driver_session_inner(engine, session))
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
            .authorize(
                token_id,
                now_ms,
                scope,
                Revision::from_wire_u64(expected_revision),
            )
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
                .authorize(
                    token_id,
                    now_ms,
                    scope,
                    Revision::from_wire_u64(expected_revision),
                )
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
            let password = password_override.unwrap_or_default();
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
            }
        };
        self.open_inner(params)
    }

    fn list_profiles_inner(&self) -> Result<Vec<BridgeProfileItem>, BridgeError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
        let inner = guard.as_ref().ok_or(BridgeError::RuntimeUnavailable)?;
        let actor = inner
            .persistence
            .as_ref()
            .ok_or_else(|| BridgeError::rejected("persistence", "configure_persistence first"))?;
        let request = ProfileListRequest::new(ProfileListFilter::new(None, None), None, 100)
            .map_err(|error| BridgeError::rejected("profile-list-request", error.to_string()))?;
        let page = actor
            .list_profiles(request)
            .map_err(|error| BridgeError::rejected("profile-list", error.to_string()))?;
        Ok(page
            .items()
            .iter()
            .map(|item| BridgeProfileItem {
                id_bytes: item.id().to_bytes().to_vec(),
                name: item.name().as_str().to_owned(),
                engine: engine_label(item.engine()).to_owned(),
                group: item.group().map(|g| g.as_str().to_owned()),
                favorite: item.favorite(),
            })
            .collect())
    }

    fn open_inner(&self, params: OpenParams) -> Result<Vec<u8>, BridgeError> {
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

        let session: Box<dyn DriverSession> = self.runtime.block_on(async {
            match engine {
                Engine::PostgreSql => {
                    let session = PostgresSession::connect_with_password(
                        &PostgresConnectConfig::new(
                            host,
                            port,
                            database,
                            user,
                            PostgresTlsMode::Disabled,
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
                            ClickHouseTlsMode::Disable,
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
                            RedisTlsMode::Disable,
                        ),
                        security,
                    )
                    .await
                    .map_err(|error| BridgeError::rejected("connect", error.to_string()))?;
                    Ok(Box::new(session) as Box<dyn DriverSession>)
                }
            }
        })??;

        self.open_driver_session_inner(engine, session)
    }

    fn open_driver_session_inner(
        &self,
        engine: Engine,
        session: Box<dyn DriverSession>,
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
        let profile_id = inner.ids.profile();
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
            let update = {
                let mut guard = self
                    .inner
                    .lock()
                    .map_err(|_| BridgeError::rejected("inner-lock", "bridge mutex poisoned"))?;
                let inner = guard.as_mut().ok_or(BridgeError::RuntimeUnavailable)?;
                match self
                    .runtime
                    .block_on(inner.service.next_update(operation_id))
                {
                    Ok(Ok(update)) => update,
                    Ok(Err(error)) => {
                        return Err(BridgeError::rejected("pump", error.to_string()));
                    }
                    Err(error) => return Err(error),
                }
            };
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
