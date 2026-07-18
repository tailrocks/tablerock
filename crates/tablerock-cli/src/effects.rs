//! Async effect executor: pure TUI effects → persistence/engine → messages.

use std::{path::PathBuf, sync::Arc};

use tablerock_core::{
    BoundedText, ByteLimit, DangerousPlaintext, Engine, EnvironmentTag, IdParts, PageKey,
    PlaintextAcknowledgement, ProfileAggregate, ProfileConnectionSnapshot, ProfileDurability,
    ProfileGroupName, ProfileId, ProfileIdentity, ProfileLimits, ProfileListFilter,
    ProfileListRequest, ProfileName, ProfileOrganization, ProfilePolicy, ProfilePreferences,
    ProfileProperty, ProfilePropertyBinding, ProfilePropertySet, ProfileSafetyMode, ProfileTag,
    ReconnectPreference, ResultStore, ResultStoreLimits, Revision, SecretSource, SecretSourceKind,
    SessionId, TlsPolicy,
};
use tablerock_engine::{
    CatalogRequest, DriverPageRequest, DriverSession, SessionRegistry,
};
use tablerock_persistence::PersistenceActor;
use tablerock_tui::{
    CatalogLevelSpec, CatalogNodeProjection, CatalogNodeStatus, CellDistinction, ConnectionDraft,
    Effect, EngineKind, FailureProjection, Message, PasswordSourceSpec, ProfilesMsg, ProjectedCell,
    RequestToken, TlsModeSpec, distinction_from_kind_label,
};
use tokio::sync::Mutex;

use crate::{RootMessageSender, projection};

static NEXT_PROFILE_LOW: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
static NEXT_SESSION_LOW: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Arbitrary-query row cap (fixed decision: result budgets).
const MAX_QUERY_ROWS: u64 = 10_000;
/// Default page size for browse/SQL streams.
const PAGE_ROWS: u32 = 500;

fn default_result_store() -> ResultStore {
    // Enough slots for multi-page pumps (10k/500 ≈ 20 pages) with pin room.
    ResultStore::new(
        ResultStoreLimits::new(32, 64, 64 * 2 * 1024 * 1024).expect("valid result store limits"),
    )
}

/// Owns process-local handles used by effect tasks.
pub struct EffectExecutor {
    persistence: Arc<Mutex<Option<PersistenceActor>>>,
    sessions: Arc<Mutex<SessionRegistry>>,
    results: Arc<Mutex<ResultStore>>,
    /// Consume-once reviewed mutation authority (handle-based apply).
    mutation_reviews: Arc<Mutex<tablerock_core::MutationReviewRegistry>>,
    ingress: RootMessageSender,
}

impl EffectExecutor {
    #[must_use]
    pub fn new(persistence: PersistenceActor, ingress: RootMessageSender) -> Self {
        Self {
            persistence: Arc::new(Mutex::new(Some(persistence))),
            sessions: Arc::new(Mutex::new(
                SessionRegistry::new(64).expect("valid session registry capacity"),
            )),
            results: Arc::new(Mutex::new(default_result_store())),
            mutation_reviews: Arc::new(Mutex::new(
                tablerock_core::MutationReviewRegistry::new(256)
                    .expect("valid mutation review registry"),
            )),
            ingress,
        }
    }

    /// Open a local-only database for the executor (default path or override).
    pub fn open_default(ingress: RootMessageSender) -> Result<Self, String> {
        let path = default_persistence_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let actor = PersistenceActor::open(&path).map_err(|error| error.to_string())?;
        Ok(Self::new(actor, ingress))
    }

    pub fn dispatch(&self, effect: Effect) {
        match effect {
            Effect::Exit => {}
            Effect::LoadProfileList {
                request_token,
                filter: _,
            } => {
                let persistence = Arc::clone(&self.persistence);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = load_profile_list(persistence, request_token).await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::CheckSessionHealth { request_token, .. } => {
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    // Engine connect path lands with plan 006; report explicit gap.
                    let _ = ingress.try_send_event(Message::Engine(
                        tablerock_tui::EngineMsg::HealthFailed {
                            request_token,
                            reason: FailureProjection::Label("not-wired".into()),
                        },
                    ));
                });
            }
            Effect::SaveConnection {
                request_token,
                draft,
            } => {
                let persistence = Arc::clone(&self.persistence);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = save_connection(persistence, request_token, draft).await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::TestConnection {
                request_token,
                draft,
            } => {
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = test_connection(request_token, draft).await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::ConnectSession {
                request_token,
                draft,
                temporary,
            } => {
                let sessions = Arc::clone(&self.sessions);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message =
                        connect_session(sessions, request_token, draft, temporary, None).await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::DisconnectSession {
                request_token,
                session_id_hex,
            } => {
                let sessions = Arc::clone(&self.sessions);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = disconnect_session(sessions, request_token, session_id_hex).await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::ConnectProfile {
                request_token,
                profile_id_hex,
            } => {
                let persistence = Arc::clone(&self.persistence);
                let sessions = Arc::clone(&self.sessions);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message =
                        connect_profile(persistence, sessions, request_token, profile_id_hex, None)
                            .await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::ResumeConnectProfile {
                request_token,
                profile_id_hex,
                password,
            } => {
                let persistence = Arc::clone(&self.persistence);
                let sessions = Arc::clone(&self.sessions);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = connect_profile(
                        persistence,
                        sessions,
                        request_token,
                        profile_id_hex,
                        Some(password),
                    )
                    .await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::ReconnectSession {
                request_token,
                draft,
                attempt,
            } => {
                let sessions = Arc::clone(&self.sessions);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = reconnect_session(sessions, request_token, draft, attempt).await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::DeleteProfile {
                request_token,
                profile_id_hex,
            } => {
                let persistence = Arc::clone(&self.persistence);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = delete_profile(persistence, request_token, profile_id_hex).await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::DeleteGroup {
                request_token,
                group_name,
            } => {
                let persistence = Arc::clone(&self.persistence);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = delete_group(persistence, request_token, group_name).await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::LoadCatalog {
                request_token,
                session_id_hex,
                context_revision,
                engine_label,
                level,
                parent_id,
            } => {
                let sessions = Arc::clone(&self.sessions);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = load_catalog(
                        sessions,
                        request_token,
                        session_id_hex,
                        context_revision,
                        engine_label,
                        level,
                        parent_id,
                    )
                    .await;
                    let _ = ingress.try_send_event(message);
                });
            }

            Effect::BrowseTable {
                request_token,
                session_id_hex,
                context_revision,
                schema,
                table,
                sort,
                filters,
                raw_where,
            } => {
                let sessions = Arc::clone(&self.sessions);
                let results = Arc::clone(&self.results);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = browse_table(
                        sessions,
                        results,
                        ingress.clone(),
                        request_token,
                        session_id_hex,
                        context_revision,
                        schema,
                        table,
                        sort,
                        filters,
                        raw_where,
                    )
                    .await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::ReviewMutations {
                request_token,
                session_id_hex,
                context_revision,
                database,
                schema,
                table,
                changes,
            } => {
                let reviews = Arc::clone(&self.mutation_reviews);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = review_mutations(
                        reviews,
                        request_token,
                        session_id_hex,
                        context_revision,
                        database,
                        schema,
                        table,
                        changes,
                    )
                    .await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::ApplyMutations {
                request_token,
                session_id_hex,
                context_revision,
                review_token_hex,
            } => {
                let sessions = Arc::clone(&self.sessions);
                let reviews = Arc::clone(&self.mutation_reviews);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = apply_mutations(
                        sessions,
                        reviews,
                        request_token,
                        session_id_hex,
                        context_revision,
                        review_token_hex,
                    )
                    .await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::LoadForeignKeys {
                request_token,
                session_id_hex,
                context_revision,
                schema,
                table,
                local_column,
                cell_value,
            } => {
                let sessions = Arc::clone(&self.sessions);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = load_foreign_keys(
                        sessions,
                        request_token,
                        session_id_hex,
                        context_revision,
                        schema,
                        table,
                        local_column,
                        cell_value,
                    )
                    .await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::LoadRelationStructure {
                request_token,
                session_id_hex,
                context_revision,
                schema,
                table,
            } => {
                let sessions = Arc::clone(&self.sessions);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = load_relation_structure(
                        sessions,
                        request_token,
                        session_id_hex,
                        context_revision,
                        schema,
                        table,
                    )
                    .await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::ExecuteTableOp {
                request_token,
                session_id_hex,
                context_revision,
                op,
                schema,
                table,
                new_table,
            } => {
                let sessions = Arc::clone(&self.sessions);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = execute_table_op(
                        sessions,
                        request_token,
                        session_id_hex,
                        context_revision,
                        op,
                        schema,
                        table,
                        new_table,
                    )
                    .await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::LoadActivity {
                request_token,
                session_id_hex,
                context_revision,
            } => {
                let sessions = Arc::clone(&self.sessions);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message =
                        load_activity(sessions, request_token, session_id_hex, context_revision)
                            .await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::SignalBackend {
                request_token,
                session_id_hex,
                context_revision,
                kind,
                pid,
            } => {
                let sessions = Arc::clone(&self.sessions);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = signal_backend(
                        sessions,
                        request_token,
                        session_id_hex,
                        context_revision,
                        kind,
                        pid,
                    )
                    .await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::ScanRedisKeys {
                request_token,
                session_id_hex,
                context_revision,
                pattern,
                count,
            } => {
                let sessions = Arc::clone(&self.sessions);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = scan_redis_keys(
                        sessions,
                        request_token,
                        session_id_hex,
                        context_revision,
                        pattern,
                        count,
                    )
                    .await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::OpenRedisKey {
                request_token,
                session_id_hex,
                context_revision,
                key,
            } => {
                let sessions = Arc::clone(&self.sessions);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = open_redis_key(
                        sessions,
                        request_token,
                        session_id_hex,
                        context_revision,
                        key,
                    )
                    .await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::LoadRedisInfo {
                request_token,
                session_id_hex,
                context_revision,
            } => {
                let sessions = Arc::clone(&self.sessions);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message =
                        load_redis_info(sessions, request_token, session_id_hex, context_revision)
                            .await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::ExportResult {
                request_token,
                path,
                format: _,
                body,
            } => {
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = export_result(request_token, path, body).await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::ExportStreamQuery {
                request_token,
                session_id_hex,
                context_revision,
                statement,
                path,
                format,
            } => {
                let sessions = Arc::clone(&self.sessions);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = export_stream_query(
                        sessions,
                        request_token,
                        session_id_hex,
                        context_revision,
                        statement,
                        path,
                        format,
                    )
                    .await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::ImportCsvApply {
                request_token,
                session_id_hex,
                context_revision,
                database,
                schema,
                table,
                path,
            } => {
                let sessions = Arc::clone(&self.sessions);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = import_csv_apply(
                        sessions,
                        request_token,
                        session_id_hex,
                        context_revision,
                        database,
                        schema,
                        table,
                        path,
                    )
                    .await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::ExecuteSql {
                request_token,
                session_id_hex,
                context_revision,
                statement,
            } => {
                let sessions = Arc::clone(&self.sessions);
                let results = Arc::clone(&self.results);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = execute_sql(
                        sessions,
                        results,
                        ingress.clone(),
                        request_token,
                        session_id_hex,
                        context_revision,
                        statement,
                        Vec::new(),
                        None::<(String, String)>, // ad-hoc SQL: no base-table identity
                    )
                    .await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::CancelQuery {
                request_token,
                session_id_hex,
            } => {
                let sessions = Arc::clone(&self.sessions);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = cancel_query(sessions, request_token, session_id_hex).await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::FetchPage {
                request_token,
                session_id_hex: _,
                context_revision,
                result_token,
                start_row,
            } => {
                let results = Arc::clone(&self.results);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message =
                        fetch_page(results, request_token, context_revision, result_token, start_row)
                            .await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::LoadHistory {
                request_token,
                search,
                limit,
            } => {
                let persistence = Arc::clone(&self.persistence);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = load_history(persistence, request_token, search, limit).await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::AppendHistory {
                request_token,
                engine_label,
                database,
                schema,
                statement,
                outcome,
                retention,
            } => {
                let persistence = Arc::clone(&self.persistence);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = append_history(
                        persistence,
                        request_token,
                        engine_label,
                        database,
                        schema,
                        statement,
                        outcome,
                        retention,
                    )
                    .await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::SaveNamedQuery {
                request_token,
                name,
                engine_label,
                statement,
            } => {
                let persistence = Arc::clone(&self.persistence);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message =
                        save_named_query(persistence, request_token, name, engine_label, statement)
                            .await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::ListNamedQueries {
                request_token,
                engine_label,
            } => {
                let persistence = Arc::clone(&self.persistence);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message =
                        list_named_queries(persistence, request_token, engine_label).await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::LoadNamedQuery {
                request_token,
                query_id,
            } => {
                let persistence = Arc::clone(&self.persistence);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = load_named_query(persistence, request_token, query_id).await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::SaveSqlFile {
                request_token,
                path,
                text,
            } => {
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = save_sql_file(request_token, path, text).await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::OpenSqlFile {
                request_token,
                path,
            } => {
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = open_sql_file(request_token, path).await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::SaveSessionIntent {
                request_token,
                profile_id_hex,
                intent_json,
            } => {
                let persistence = Arc::clone(&self.persistence);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message =
                        save_session_intent(persistence, request_token, profile_id_hex, intent_json)
                            .await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::LoadSessionIntent {
                request_token,
                profile_id_hex,
            } => {
                let persistence = Arc::clone(&self.persistence);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message =
                        load_session_intent(persistence, request_token, profile_id_hex).await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::CopyToClipboard {
                request_token,
                text,
            } => {
                let ingress = self.ingress.clone();
                let bytes = text.len();
                // Best-effort OSC 52 to stdout (terminal clipboard). Failures
                // still report byte count; pure formatters are the product gate.
                let _ = write_osc52_clipboard(&text);
                let _ = ingress.try_send_event(Message::Engine(
                    tablerock_tui::EngineMsg::ClipboardCopied {
                        request_token,
                        bytes,
                    },
                ));
            }
            Effect::SaveColumnLayout {
                request_token,
                profile_id_hex,
                database,
                schema,
                table,
                layout_json,
            } => {
                let persistence = Arc::clone(&self.persistence);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = save_column_layout(
                        persistence,
                        request_token,
                        profile_id_hex,
                        database,
                        schema,
                        table,
                        layout_json,
                    )
                    .await;
                    let _ = ingress.try_send_event(message);
                });
            }
            Effect::LoadColumnLayout {
                request_token,
                profile_id_hex,
                database,
                schema,
                table,
            } => {
                let persistence = Arc::clone(&self.persistence);
                let ingress = self.ingress.clone();
                tokio::task::spawn_local(async move {
                    let message = load_column_layout(
                        persistence,
                        request_token,
                        profile_id_hex,
                        database,
                        schema,
                        table,
                    )
                    .await;
                    let _ = ingress.try_send_event(message);
                });
            }
        }
    }
}

/// OSC 52 clipboard write: ESC ] 52 ; c ; <base64> BEL
fn write_osc52_clipboard(text: &str) -> std::io::Result<()> {
    use std::io::Write;
    // Minimal base64 (std-only).
    let b64 = base64_encode(text.as_bytes());
    let mut out = std::io::stdout().lock();
    write!(out, "\x1b]52;c;{b64}\x07")?;
    out.flush()
}

fn base64_encode(input: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = chunk.get(1).copied().map(u32::from).unwrap_or(0);
        let b2 = chunk.get(2).copied().map(u32::from).unwrap_or(0);
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(T[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(T[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

async fn load_history(
    persistence: Arc<Mutex<Option<PersistenceActor>>>,
    request_token: RequestToken,
    search: Option<String>,
    limit: u32,
) -> Message {
    use tablerock_persistence::HistoryEntry;
    let joined = tokio::task::spawn_blocking(move || {
        let guard = persistence.blocking_lock();
        let Some(actor) = guard.as_ref() else {
            return Err("persistence unavailable".to_owned());
        };
        actor
            .list_history(search, limit)
            .map_err(|error| error.to_string())
    })
    .await;
    match joined {
        Ok(Ok(entries)) => Message::Engine(tablerock_tui::EngineMsg::HistoryLoaded {
            request_token,
            entries: entries.into_iter().map(history_row).collect(),
        }),
        Ok(Err(label)) => Message::Engine(tablerock_tui::EngineMsg::HistoryFailed {
            request_token,
            reason: FailureProjection::Label(label),
        }),
        Err(_) => Message::Engine(tablerock_tui::EngineMsg::HistoryFailed {
            request_token,
            reason: FailureProjection::Label("history task failed".into()),
        }),
    }
}

async fn append_history(
    persistence: Arc<Mutex<Option<PersistenceActor>>>,
    request_token: RequestToken,
    engine_label: String,
    database: String,
    schema: Option<String>,
    statement: String,
    outcome: String,
    retention: String,
) -> Message {
    use tablerock_core::Engine;
    use tablerock_persistence::{HistoryAppend, HistoryOutcomeClass, HistoryRetention};
    let engine = match engine_label.as_str() {
        "ClickHouse" => Engine::ClickHouse,
        "Redis" => Engine::Redis,
        _ => Engine::PostgreSql,
    };
    let outcome = match outcome.as_str() {
        "cancelled" => HistoryOutcomeClass::Cancelled,
        "failed" => HistoryOutcomeClass::Failed,
        "disconnected" => HistoryOutcomeClass::Disconnected,
        "completed" => HistoryOutcomeClass::Completed,
        _ => HistoryOutcomeClass::Unknown,
    };
    let retention = match retention.as_str() {
        "metadata" => HistoryRetention::MetadataOnly,
        "private" => HistoryRetention::Private,
        _ => HistoryRetention::Full,
    };
    let joined = tokio::task::spawn_blocking(move || {
        let guard = persistence.blocking_lock();
        let Some(actor) = guard.as_ref() else {
            return Err("persistence unavailable".to_owned());
        };
        actor
            .append_history(HistoryAppend {
                engine,
                database_name: database,
                schema_name: schema,
                statement_text: statement,
                outcome,
                retention,
            })
            .map_err(|error| error.to_string())
    })
    .await;
    match joined {
        Ok(Ok(history_id)) => Message::Engine(tablerock_tui::EngineMsg::HistoryAppended {
            request_token,
            history_id,
        }),
        Ok(Err(label)) => Message::Engine(tablerock_tui::EngineMsg::HistoryFailed {
            request_token,
            reason: FailureProjection::Label(label),
        }),
        Err(_) => Message::Engine(tablerock_tui::EngineMsg::HistoryFailed {
            request_token,
            reason: FailureProjection::Label("history append task failed".into()),
        }),
    }
}

async fn save_named_query(
    persistence: Arc<Mutex<Option<PersistenceActor>>>,
    request_token: RequestToken,
    name: String,
    engine_label: String,
    statement: String,
) -> Message {
    use tablerock_core::Engine;
    use tablerock_persistence::SavedQueryUpsert;
    let engine = match engine_label.as_str() {
        "ClickHouse" => Engine::ClickHouse,
        "Redis" => Engine::Redis,
        _ => Engine::PostgreSql,
    };
    let joined = tokio::task::spawn_blocking(move || {
        let guard = persistence.blocking_lock();
        let Some(actor) = guard.as_ref() else {
            return Err("persistence unavailable".to_owned());
        };
        actor
            .upsert_saved_query(SavedQueryUpsert {
                name: name.clone(),
                engine,
                statement_text: statement,
            })
            .map(|query_id| (query_id, name))
            .map_err(|e| e.to_string())
    })
    .await;
    match joined {
        Ok(Ok((query_id, name))) => Message::Engine(tablerock_tui::EngineMsg::NamedQuerySaved {
            request_token,
            query_id,
            name,
        }),
        Ok(Err(label)) => Message::Engine(tablerock_tui::EngineMsg::SqlFileFailed {
            request_token,
            reason: FailureProjection::Label(label),
        }),
        Err(_) => Message::Engine(tablerock_tui::EngineMsg::SqlFileFailed {
            request_token,
            reason: FailureProjection::Label("save query task failed".into()),
        }),
    }
}

async fn list_named_queries(
    persistence: Arc<Mutex<Option<PersistenceActor>>>,
    request_token: RequestToken,
    engine_label: String,
) -> Message {
    use tablerock_core::Engine;
    let engine = match engine_label.as_str() {
        "ClickHouse" => Engine::ClickHouse,
        "Redis" => Engine::Redis,
        _ => Engine::PostgreSql,
    };
    let joined = tokio::task::spawn_blocking(move || {
        let guard = persistence.blocking_lock();
        let Some(actor) = guard.as_ref() else {
            return Err("persistence unavailable".to_owned());
        };
        actor
            .list_saved_queries(Some(engine))
            .map_err(|e| e.to_string())
    })
    .await;
    match joined {
        Ok(Ok(entries)) => Message::Engine(tablerock_tui::EngineMsg::NamedQueriesLoaded {
            request_token,
            entries: entries
                .into_iter()
                .map(|q| {
                    let engine_label = match q.engine {
                        Engine::PostgreSql => "PostgreSQL",
                        Engine::ClickHouse => "ClickHouse",
                        Engine::Redis => "Redis",
                    }
                    .to_owned();
                    tablerock_tui::SavedQueryRow {
                        query_id: q.query_id,
                        name: q.name,
                        engine_label,
                        statement_preview: q.statement_text.chars().take(120).collect(),
                    }
                })
                .collect(),
        }),
        Ok(Err(label)) => Message::Engine(tablerock_tui::EngineMsg::SqlFileFailed {
            request_token,
            reason: FailureProjection::Label(label),
        }),
        Err(_) => Message::Engine(tablerock_tui::EngineMsg::SqlFileFailed {
            request_token,
            reason: FailureProjection::Label("list queries task failed".into()),
        }),
    }
}

async fn load_named_query(
    persistence: Arc<Mutex<Option<PersistenceActor>>>,
    request_token: RequestToken,
    query_id: i64,
) -> Message {
    let joined = tokio::task::spawn_blocking(move || {
        let guard = persistence.blocking_lock();
        let Some(actor) = guard.as_ref() else {
            return Err("persistence unavailable".to_owned());
        };
        actor.get_saved_query(query_id).map_err(|e| e.to_string())
    })
    .await;
    match joined {
        Ok(Ok(Some(q))) => Message::Engine(tablerock_tui::EngineMsg::NamedQueryLoaded {
            request_token,
            name: q.name,
            statement: q.statement_text,
        }),
        Ok(Ok(None)) => Message::Engine(tablerock_tui::EngineMsg::SqlFileFailed {
            request_token,
            reason: FailureProjection::Label("query not found".into()),
        }),
        Ok(Err(label)) => Message::Engine(tablerock_tui::EngineMsg::SqlFileFailed {
            request_token,
            reason: FailureProjection::Label(label),
        }),
        Err(_) => Message::Engine(tablerock_tui::EngineMsg::SqlFileFailed {
            request_token,
            reason: FailureProjection::Label("load query task failed".into()),
        }),
    }
}

async fn save_sql_file(request_token: RequestToken, path: String, text: String) -> Message {
    use std::time::UNIX_EPOCH;
    use tablerock_persistence::write_sql_file_atomic;
    let joined = tokio::task::spawn_blocking(move || {
        write_sql_file_atomic(std::path::Path::new(&path), &text).map(|facts| {
            let mtime_secs = facts
                .mtime
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs());
            (facts.path.display().to_string(), mtime_secs, facts.len)
        })
    })
    .await;
    match joined {
        Ok(Ok((path, mtime_secs, len))) => Message::Engine(tablerock_tui::EngineMsg::SqlFileSaved {
            request_token,
            path,
            mtime_secs,
            len,
        }),
        Ok(Err(_)) => Message::Engine(tablerock_tui::EngineMsg::SqlFileFailed {
            request_token,
            reason: FailureProjection::Label("sql file write failed".into()),
        }),
        Err(_) => Message::Engine(tablerock_tui::EngineMsg::SqlFileFailed {
            request_token,
            reason: FailureProjection::Label("sql file write task failed".into()),
        }),
    }
}

async fn open_sql_file(request_token: RequestToken, path: String) -> Message {
    use std::time::UNIX_EPOCH;
    use tablerock_persistence::read_sql_file;
    let joined = tokio::task::spawn_blocking(move || {
        read_sql_file(std::path::Path::new(&path)).map(|(text, facts)| {
            let mtime_secs = facts
                .mtime
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs());
            (
                facts.path.display().to_string(),
                text,
                mtime_secs,
                facts.len,
            )
        })
    })
    .await;
    match joined {
        Ok(Ok((path, text, mtime_secs, len))) => {
            Message::Engine(tablerock_tui::EngineMsg::SqlFileOpened {
                request_token,
                path,
                text,
                mtime_secs,
                len,
            })
        }
        Ok(Err(_)) => Message::Engine(tablerock_tui::EngineMsg::SqlFileFailed {
            request_token,
            reason: FailureProjection::Label("sql file read failed".into()),
        }),
        Err(_) => Message::Engine(tablerock_tui::EngineMsg::SqlFileFailed {
            request_token,
            reason: FailureProjection::Label("sql file read task failed".into()),
        }),
    }
}

async fn save_session_intent(
    persistence: Arc<Mutex<Option<PersistenceActor>>>,
    request_token: RequestToken,
    profile_id_hex: String,
    intent_json: String,
) -> Message {
    use tablerock_core::ProfileId;
    let joined = tokio::task::spawn_blocking(move || {
        let id: ProfileId = profile_id_hex
            .parse()
            .map_err(|_| "invalid profile id".to_owned())?;
        let guard = persistence.blocking_lock();
        let Some(actor) = guard.as_ref() else {
            return Err("persistence unavailable".to_owned());
        };
        actor
            .put_session_intent(id, intent_json)
            .map_err(|e| e.to_string())
    })
    .await;
    match joined {
        Ok(Ok(())) => Message::Engine(tablerock_tui::EngineMsg::SessionIntentSaved { request_token }),
        Ok(Err(label)) => Message::Engine(tablerock_tui::EngineMsg::SessionIntentFailed {
            request_token,
            reason: FailureProjection::Label(label),
        }),
        Err(_) => Message::Engine(tablerock_tui::EngineMsg::SessionIntentFailed {
            request_token,
            reason: FailureProjection::Label("save intent task failed".into()),
        }),
    }
}

async fn load_session_intent(
    persistence: Arc<Mutex<Option<PersistenceActor>>>,
    request_token: RequestToken,
    profile_id_hex: String,
) -> Message {
    use tablerock_core::ProfileId;
    let joined = tokio::task::spawn_blocking(move || {
        let id: ProfileId = profile_id_hex
            .parse()
            .map_err(|_| "invalid profile id".to_owned())?;
        let guard = persistence.blocking_lock();
        let Some(actor) = guard.as_ref() else {
            return Err("persistence unavailable".to_owned());
        };
        actor
            .get_session_intent(id)
            .map_err(|e| e.to_string())
            .map(|opt| opt.map(|r| r.intent_json))
    })
    .await;
    match joined {
        Ok(Ok(intent_json)) => Message::Engine(tablerock_tui::EngineMsg::SessionIntentLoaded {
            request_token,
            intent_json,
        }),
        Ok(Err(label)) => Message::Engine(tablerock_tui::EngineMsg::SessionIntentFailed {
            request_token,
            reason: FailureProjection::Label(label),
        }),
        Err(_) => Message::Engine(tablerock_tui::EngineMsg::SessionIntentFailed {
            request_token,
            reason: FailureProjection::Label("load intent task failed".into()),
        }),
    }
}

async fn save_column_layout(
    persistence: Arc<Mutex<Option<PersistenceActor>>>,
    request_token: RequestToken,
    profile_id_hex: String,
    database: String,
    schema: String,
    table: String,
    layout_json: String,
) -> Message {
    use tablerock_core::ProfileId;
    use tablerock_persistence::ColumnLayoutKey;
    let joined = tokio::task::spawn_blocking(move || {
        let id: ProfileId = profile_id_hex
            .parse()
            .map_err(|_| "invalid profile id".to_owned())?;
        let guard = persistence.blocking_lock();
        let Some(actor) = guard.as_ref() else {
            return Err("persistence unavailable".to_owned());
        };
        actor
            .put_column_layout(
                ColumnLayoutKey {
                    profile_id: id,
                    database,
                    schema,
                    table,
                },
                layout_json,
            )
            .map_err(|e| e.to_string())
    })
    .await;
    match joined {
        Ok(Ok(())) => Message::Engine(tablerock_tui::EngineMsg::ColumnLayoutSaved { request_token }),
        Ok(Err(label)) => Message::Engine(tablerock_tui::EngineMsg::ColumnLayoutFailed {
            request_token,
            reason: FailureProjection::Label(label),
        }),
        Err(_) => Message::Engine(tablerock_tui::EngineMsg::ColumnLayoutFailed {
            request_token,
            reason: FailureProjection::Label("save column layout task failed".into()),
        }),
    }
}

async fn load_column_layout(
    persistence: Arc<Mutex<Option<PersistenceActor>>>,
    request_token: RequestToken,
    profile_id_hex: String,
    database: String,
    schema: String,
    table: String,
) -> Message {
    use tablerock_core::ProfileId;
    use tablerock_persistence::ColumnLayoutKey;
    let joined = tokio::task::spawn_blocking(move || {
        let id: ProfileId = profile_id_hex
            .parse()
            .map_err(|_| "invalid profile id".to_owned())?;
        let guard = persistence.blocking_lock();
        let Some(actor) = guard.as_ref() else {
            return Err("persistence unavailable".to_owned());
        };
        actor
            .get_column_layout(ColumnLayoutKey {
                profile_id: id,
                database,
                schema,
                table,
            })
            .map_err(|e| e.to_string())
            .map(|opt| opt.map(|r| r.layout_json))
    })
    .await;
    match joined {
        Ok(Ok(layout_json)) => Message::Engine(tablerock_tui::EngineMsg::ColumnLayoutLoaded {
            request_token,
            layout_json,
        }),
        Ok(Err(label)) => Message::Engine(tablerock_tui::EngineMsg::ColumnLayoutFailed {
            request_token,
            reason: FailureProjection::Label(label),
        }),
        Err(_) => Message::Engine(tablerock_tui::EngineMsg::ColumnLayoutFailed {
            request_token,
            reason: FailureProjection::Label("load column layout task failed".into()),
        }),
    }
}

fn history_row(
    entry: tablerock_persistence::HistoryEntry,
) -> tablerock_tui::HistoryRowProjection {
    use tablerock_core::Engine;
    let engine_label = match entry.engine {
        Engine::PostgreSql => "PostgreSQL",
        Engine::ClickHouse => "ClickHouse",
        Engine::Redis => "Redis",
    }
    .to_owned();
    let preview = entry
        .statement_text
        .as_deref()
        .map(|s| {
            let one_line: String = s.chars().take(120).collect();
            one_line
        })
        .unwrap_or_else(|| "(no text)".into());
    tablerock_tui::HistoryRowProjection {
        history_id: entry.history_id,
        engine_label,
        database: entry.database_name,
        schema: entry.schema_name,
        statement_preview: preview,
        outcome: entry.outcome.as_str().to_owned(),
        created_at: entry.created_at,
    }
}

async fn load_profile_list(
    persistence: Arc<Mutex<Option<PersistenceActor>>>,
    request_token: RequestToken,
) -> Message {
    let joined = tokio::task::spawn_blocking(move || {
        let guard = persistence.blocking_lock();
        let Some(actor) = guard.as_ref() else {
            return Err("persistence unavailable".to_owned());
        };
        let request = ProfileListRequest::new(ProfileListFilter::default(), None, 100)
            .map_err(|error| error.to_string())?;
        actor
            .list_profiles(request)
            .map_err(|error| error.to_string())
    })
    .await;
    match joined {
        Ok(Ok(page)) => {
            let items = page.items().iter().map(projection::profile_row).collect();
            Message::Profiles(ProfilesMsg::ListLoaded {
                request_token,
                items,
            })
        }
        Ok(Err(label)) => Message::Profiles(ProfilesMsg::ListFailed {
            request_token,
            reason: FailureProjection::Label(label),
        }),
        Err(_) => Message::Profiles(ProfilesMsg::ListFailed {
            request_token,
            reason: FailureProjection::Label("task-failed".into()),
        }),
    }
}

async fn test_connection(request_token: RequestToken, draft: ConnectionDraft) -> Message {
    match open_described_session(draft).await {
        Ok((session, identity, elapsed_millis, tunnel)) => {
            let _ = session.shutdown().await;
            drop(tunnel);
            Message::Engine(tablerock_tui::EngineMsg::TestOk {
                request_token,
                identity,
                elapsed_millis,
            })
        }
        Err(label) => Message::Engine(tablerock_tui::EngineMsg::TestFailed {
            request_token,
            reason: FailureProjection::Label(label),
        }),
    }
}

async fn connect_session(
    sessions: Arc<Mutex<SessionRegistry>>,
    request_token: RequestToken,
    draft: ConnectionDraft,
    temporary: bool,
    profile_id_hex: Option<String>,
) -> Message {
    let engine_label = match draft.engine {
        EngineKind::PostgreSql => "PostgreSQL",
        EngineKind::ClickHouse => "ClickHouse",
        EngineKind::Redis => "Redis",
    }
    .to_owned();
    match open_described_session(draft).await {
        Ok((session, identity, _elapsed, tunnel)) => {
            let session_id = match mint_session_id() {
                Ok(id) => id,
                Err(label) => {
                    let _ = session.shutdown().await;
                    drop(tunnel);
                    return Message::Engine(tablerock_tui::EngineMsg::ConnectFailed {
                        request_token,
                        reason: FailureProjection::Label(label),
                    });
                }
            };
            let mut registry = sessions.lock().await;
            match registry.register_with_tunnel(session_id, session, tunnel) {
                Ok(_) => Message::Engine(tablerock_tui::EngineMsg::ConnectOk {
                    request_token,
                    session_id_hex: session_id.to_string(),
                    identity,
                    temporary,
                    engine_label,
                    profile_id_hex: if temporary { None } else { profile_id_hex },
                }),
                Err(error) => Message::Engine(tablerock_tui::EngineMsg::ConnectFailed {
                    request_token,
                    reason: FailureProjection::Label(error.to_string()),
                }),
            }
        }
        Err(label) => Message::Engine(tablerock_tui::EngineMsg::ConnectFailed {
            request_token,
            reason: FailureProjection::Label(label),
        }),
    }
}

async fn disconnect_session(
    sessions: Arc<Mutex<SessionRegistry>>,
    request_token: RequestToken,
    session_id_hex: String,
) -> Message {
    let session_id = match session_id_hex.parse::<SessionId>() {
        Ok(id) => id,
        Err(_) => {
            return Message::Engine(tablerock_tui::EngineMsg::DisconnectFailed {
                request_token,
                reason: FailureProjection::Label("invalid session id".into()),
            });
        }
    };
    let mut registry = sessions.lock().await;
    match registry.disconnect(session_id).await {
        Ok(()) => Message::Engine(tablerock_tui::EngineMsg::DisconnectOk {
            request_token,
            session_id_hex,
        }),
        Err(error) => Message::Engine(tablerock_tui::EngineMsg::DisconnectFailed {
            request_token,
            reason: FailureProjection::Label(error.to_string()),
        }),
    }
}

fn mint_session_id() -> Result<SessionId, String> {
    let low = NEXT_SESSION_LOW.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    SessionId::from_parts(IdParts::new(1, low).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

async fn load_catalog(
    sessions: Arc<Mutex<SessionRegistry>>,
    request_token: RequestToken,
    session_id_hex: String,
    context_revision: u64,
    engine_label: String,
    level: CatalogLevelSpec,
    parent_id: Option<String>,
) -> Message {
    use tablerock_core::{BoundedText, ByteLimit, PageLimits};
    let session_id = match session_id_hex.parse::<SessionId>() {
        Ok(id) => id,
        Err(_) => {
            return Message::Engine(tablerock_tui::EngineMsg::CatalogFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label("invalid session id".into()),
            });
        }
    };
    let session = {
        let registry = sessions.lock().await;
        registry.session(session_id)
    };
    let Some(session) = session else {
        return Message::Engine(tablerock_tui::EngineMsg::CatalogFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label("session not registered".into()),
        });
    };
    let text = |value: &str| {
        BoundedText::copy_from_str(value, ByteLimit::new(128)).map_err(|e| e.to_string())
    };
    let limits = PageLimits::new(256, 1, 64 * 1024, 256);
    let request = match (&engine_label[..], &level) {
        ("PostgreSQL", CatalogLevelSpec::Root) => CatalogRequest::PostgreSqlDatabases { limits },
        ("PostgreSQL", CatalogLevelSpec::Schemas { database }) => match text(database) {
            Ok(database) => CatalogRequest::PostgreSqlSchemas { database, limits },
            Err(label) => {
                return Message::Engine(tablerock_tui::EngineMsg::CatalogFailed {
                    request_token,
                    context_revision,
                    reason: FailureProjection::Label(label),
                });
            }
        },
        ("PostgreSQL", CatalogLevelSpec::Relations { database, schema }) => {
            match (text(database), text(schema)) {
                (Ok(database), Ok(schema)) => CatalogRequest::PostgreSqlRelations {
                    database,
                    schema,
                    limits,
                },
                (Err(label), _) | (_, Err(label)) => {
                    return Message::Engine(tablerock_tui::EngineMsg::CatalogFailed {
                        request_token,
                        context_revision,
                        reason: FailureProjection::Label(label),
                    });
                }
            }
        }
        ("ClickHouse", CatalogLevelSpec::Root) => CatalogRequest::ClickHouseDatabases { limits },
        ("ClickHouse", CatalogLevelSpec::Objects { database }) => match text(database) {
            Ok(database) => CatalogRequest::ClickHouseObjects { database, limits },
            Err(label) => {
                return Message::Engine(tablerock_tui::EngineMsg::CatalogFailed {
                    request_token,
                    context_revision,
                    reason: FailureProjection::Label(label),
                });
            }
        },
        ("Redis", CatalogLevelSpec::Root) => CatalogRequest::RedisLogicalDatabases { limits },
        _ => {
            return Message::Engine(tablerock_tui::EngineMsg::CatalogFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label("catalog level unsupported".into()),
            });
        }
    };
    match session.catalog(request).await {
        Ok(subtree) => {
            let truncated = !subtree.complete()
                || matches!(
                    subtree.exactness(),
                    tablerock_engine::CatalogExactness::Truncated
                );
            let parent_prefix = parent_id.as_deref().unwrap_or("");
            let parent_depth = if parent_id.is_some() {
                parent_prefix.matches('/').count() as u16 + 1
            } else {
                0
            };
            let nodes: Vec<CatalogNodeProjection> = subtree
                .nodes()
                .iter()
                .map(|seed| {
                    let kind_label = catalog_kind_label(seed.kind());
                    let name = seed.name().to_owned();
                    let id = if parent_prefix.is_empty() {
                        name.clone()
                    } else {
                        format!("{parent_prefix}/{name}")
                    };
                    let branch = !matches!(
                        seed.children(),
                        tablerock_core::CatalogChildrenState::NotApplicable
                    ) && !matches!(
                        seed.kind(),
                        tablerock_core::CatalogNodeKind::PostgreSqlObject(_)
                            | tablerock_core::CatalogNodeKind::ClickHouseObject(_)
                            | tablerock_core::CatalogNodeKind::RedisKey(_)
                    );
                    CatalogNodeProjection {
                        id,
                        label: name,
                        kind_label: kind_label.into(),
                        depth: parent_depth,
                        branch,
                        expanded: false,
                        status: CatalogNodeStatus::Ready,
                    }
                })
                .collect();
            Message::Engine(tablerock_tui::EngineMsg::CatalogLoaded {
                request_token,
                context_revision,
                parent_id,
                nodes,
                truncated,
            })
        }
        Err(error) => Message::Engine(tablerock_tui::EngineMsg::CatalogFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label(error.to_string()),
        }),
    }
}

async fn browse_table(
    sessions: Arc<Mutex<SessionRegistry>>,
    results: Arc<Mutex<ResultStore>>,
    ingress: RootMessageSender,
    request_token: RequestToken,
    session_id_hex: String,
    context_revision: u64,
    schema: String,
    table: String,
    sort: Vec<(String, String)>,
    filters: Vec<(String, String, Option<String>)>,
    raw_where: Option<String>,
) -> Message {
    use tablerock_engine::{
        BrowsePlan, FilterOperator, FilterValue, SortDirection, SortKey, TypedCondition,
    };
    let mut plan = BrowsePlan {
        schema,
        table,
        sort: sort
            .into_iter()
            .filter_map(|(column, dir)| {
                let direction = match dir.as_str() {
                    "desc" | "Desc" | "DESC" => SortDirection::Desc,
                    "asc" | "Asc" | "ASC" => SortDirection::Asc,
                    _ => return None,
                };
                Some(SortKey { column, direction })
            })
            .collect(),
        filters: Vec::new(),
        raw_where,
        limit: PAGE_ROWS,
        offset: 0,
    };
    for (column, op, value) in filters {
        let operator = match op.to_ascii_lowercase().as_str() {
            "eq" | "=" => FilterOperator::Eq,
            "ne" | "<>" | "!=" => FilterOperator::Ne,
            "lt" | "<" => FilterOperator::Lt,
            "le" | "<=" => FilterOperator::Le,
            "gt" | ">" => FilterOperator::Gt,
            "ge" | ">=" => FilterOperator::Ge,
            "like" => FilterOperator::Like,
            "ilike" => FilterOperator::ILike,
            "isnull" | "is_null" => FilterOperator::IsNull,
            "isnotnull" | "is_not_null" => FilterOperator::IsNotNull,
            _ => {
                return Message::Engine(tablerock_tui::EngineMsg::GridFailed {
                    request_token,
                    context_revision,
                    reason: FailureProjection::Label(format!("unknown filter operator: {op}")),
                });
            }
        };
        let value = if operator.needs_value() {
            let Some(v) = value else {
                return Message::Engine(tablerock_tui::EngineMsg::GridFailed {
                    request_token,
                    context_revision,
                    reason: FailureProjection::Label("filter value required".into()),
                });
            };
            // Prefer integer when it parses; else text (boolean true/false).
            let fv = if let Ok(n) = v.parse::<i64>() {
                FilterValue::Integer(n)
            } else if v.eq_ignore_ascii_case("true") {
                FilterValue::Boolean(true)
            } else if v.eq_ignore_ascii_case("false") {
                FilterValue::Boolean(false)
            } else if let Ok(n) = v.parse::<f64>() {
                FilterValue::Float(n)
            } else {
                FilterValue::Text(v)
            };
            Some(fv)
        } else {
            None
        };
        plan.filters.push(TypedCondition {
            column,
            operator,
            value,
        });
    }
    let rendered = match plan.render_sql() {
        Ok(r) => r,
        Err(error) => {
            return Message::Engine(tablerock_tui::EngineMsg::GridFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(error.to_string()),
            });
        }
    };
    let schema_for_pk = plan.schema.clone();
    let table_for_pk = plan.table.clone();
    execute_sql(
        sessions,
        results,
        ingress,
        request_token,
        session_id_hex,
        context_revision,
        rendered.sql,
        rendered.parameters,
        // PK proof runs inside execute when session is already resolved.
        Some((schema_for_pk, table_for_pk)),
    )
    .await
}

/// Pump-and-store: stream pages into ResultStore up to the query cap; surface
/// the first page before completion so the grid can paint early. Further
/// pages are projected via FetchPage (no OFFSET re-query).
async fn execute_sql(
    sessions: Arc<Mutex<SessionRegistry>>,
    results: Arc<Mutex<ResultStore>>,
    ingress: RootMessageSender,
    request_token: RequestToken,
    session_id_hex: String,
    context_revision: u64,
    statement: String,
    parameters: Vec<tablerock_engine::FilterValue>,
    // When set (browse), load primary-key columns for editability.
    identity_relation: Option<(String, String)>,
) -> Message {
    use tablerock_core::{
        Engine as CoreEngine, IdParts, PageIdentity, PageLimits, ResultId, Revision, StatementText,
    };
    let session_id = match session_id_hex.parse::<SessionId>() {
        Ok(id) => id,
        Err(_) => {
            return Message::Engine(tablerock_tui::EngineMsg::GridFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label("invalid session id".into()),
            });
        }
    };
    let session = {
        let registry = sessions.lock().await;
        registry.session(session_id)
    };
    let Some(session) = session else {
        return Message::Engine(tablerock_tui::EngineMsg::GridFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label("session not registered".into()),
        });
    };
    let identity_columns = match identity_relation {
        Some((schema, table)) if session.engine() == CoreEngine::PostgreSql => {
            fetch_primary_key_columns(session.as_ref(), &schema, &table).await
        }
        _ => None,
    };
    let statement = match StatementText::new(statement) {
        Ok(s) => s,
        Err(error) => {
            return Message::Engine(tablerock_tui::EngineMsg::GridFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(error.to_string()),
            });
        }
    };
    let limits = PageLimits::new(PAGE_ROWS, 64, 2 * 1024 * 1024, 64 * 1024);
    let mut stream = match session
        .start_page_stream(DriverPageRequest::PostgreSqlStatement {
            statement,
            parameters,
            limits,
            max_cell_bytes: 64 * 1024,
        })
        .await
    {
        Ok(stream) => stream,
        Err(error) => {
            return Message::Engine(tablerock_tui::EngineMsg::GridFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(error.to_string()),
            });
        }
    };
    let low = request_token.max(1);
    let result_id =
        ResultId::from_parts(IdParts::new(1, low).expect("id parts")).expect("result id");
    let identity = PageIdentity::new(result_id, Revision::INITIAL, CoreEngine::PostgreSql);
    {
        let mut store = results.lock().await;
        let _ = store.open_result(identity);
    }

    let mut start_row = 0_u64;
    let mut first_sent = false;
    let mut hit_cap = false;
    let mut total_rows = 0_u64;

    loop {
        if start_row >= MAX_QUERY_ROWS {
            hit_cap = true;
            break;
        }
        match stream.next_page(identity, start_row).await {
            Ok(Some(page)) => {
                let row_count = u64::from(page.envelope().row_count());
                let page_start = page.envelope().start_row();
                {
                    let mut store = results.lock().await;
                    match store.admit(page.clone()) {
                        Ok(outcome) => {
                            // Pin the first page so the resident viewport is not
                            // LRU-evicted while later pages stream in.
                            if page_start == 0 {
                                let _ = store.set_pinned(outcome.admitted(), true);
                            }
                        }
                        Err(error) => {
                            return Message::Engine(tablerock_tui::EngineMsg::GridFailed {
                                request_token,
                                context_revision,
                                reason: FailureProjection::Label(error.to_string()),
                            });
                        }
                    }
                }
                total_rows = total_rows.max(page_start.saturating_add(row_count));
                if !first_sent {
                    // First rows before stream completion (Phase 4 exit).
                    let msg = project_page_message(
                        request_token,
                        context_revision,
                        page,
                        false,
                        identity_columns.clone(),
                    );
                    let _ = ingress.try_send_event(msg);
                    first_sent = true;
                }
                start_row = page_start.saturating_add(row_count);
                if start_row >= MAX_QUERY_ROWS {
                    hit_cap = true;
                    break;
                }
            }
            Ok(None) => break,
            Err(error) => {
                let label = error.to_string();
                // Honest race: server-confirmed cancel vs other stream failures.
                if label.contains("cancel") {
                    return Message::Engine(tablerock_tui::EngineMsg::GridCancelled {
                        request_token,
                        label: "server confirmed cancelled".into(),
                    });
                }
                return Message::Engine(tablerock_tui::EngineMsg::GridFailed {
                    request_token,
                    context_revision,
                    reason: FailureProjection::Label(label),
                });
            }
        }
    }

    if !first_sent {
        // Empty result set.
        return Message::Engine(tablerock_tui::EngineMsg::GridPage {
            request_token,
            context_revision,
            start_row: 0,
            columns: Vec::new(),
            cells: Vec::new(),
            row_count: 0,
            totals_exact: Some(0),
            totals_estimated: None,
            bytes: 0,
            truncated: false,
            complete: true,
            identity_columns,
        });
    }

    Message::Engine(tablerock_tui::EngineMsg::GridStreamComplete {
        request_token,
        context_revision,
        rows_loaded: total_rows,
        truncated: hit_cap,
    })
}

async fn fetch_page(
    results: Arc<Mutex<ResultStore>>,
    request_token: RequestToken,
    context_revision: u64,
    result_token: RequestToken,
    start_row: u64,
) -> Message {
    use tablerock_core::{IdParts, ResultId, Revision};
    let low = result_token.max(1);
    let result_id =
        ResultId::from_parts(IdParts::new(1, low).expect("id parts")).expect("result id");
    let key = PageKey::new(result_id, Revision::INITIAL, start_row);
    let page = {
        let mut store = results.lock().await;
        // Pin the requested page (viewport) so LRU cannot evict it.
        let pinned = store.set_pinned(key, true);
        if !pinned {
            // Page not admitted (evicted or never pumped) — honest miss.
            return Message::Engine(tablerock_tui::EngineMsg::GridFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(format!(
                    "page at row {start_row} not resident"
                )),
            });
        }
        store.get(key).cloned()
    };
    let Some(page) = page else {
        return Message::Engine(tablerock_tui::EngineMsg::GridFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label(format!("page at row {start_row} not resident")),
        });
    };
    // complete=false: FetchPage only swaps the resident window; terminal
    // completion already arrived (or will) via GridStreamComplete.
    project_page_message(request_token, context_revision, page, false, None)
}

async fn cancel_query(
    sessions: Arc<Mutex<SessionRegistry>>,
    request_token: RequestToken,
    session_id_hex: String,
) -> Message {
    use tablerock_core::{IdParts, OperationId};
    let session_id = match session_id_hex.parse::<SessionId>() {
        Ok(id) => id,
        Err(_) => {
            return Message::Engine(tablerock_tui::EngineMsg::GridFailed {
                request_token,
                context_revision: 0,
                reason: FailureProjection::Label("invalid session id".into()),
            });
        }
    };
    let session = {
        let registry = sessions.lock().await;
        registry.session(session_id)
    };
    let Some(session) = session else {
        return Message::Engine(tablerock_tui::EngineMsg::GridFailed {
            request_token,
            context_revision: 0,
            reason: FailureProjection::Label("session not registered".into()),
        });
    };
    let low = request_token.max(1);
    let op = OperationId::from_parts(IdParts::new(1, low).expect("id parts")).expect("op id");
    let dispatch = session.cancel(op).await;
    // Dispatch fact only — terminal race outcome arrives via the stream task
    // (GridCancelled / GridFailed / GridStreamComplete).
    use tablerock_core::CancelDispatch;
    let dispatch = match dispatch {
        CancelDispatch::RequestSent => "request_sent",
        CancelDispatch::PreventedBeforeDispatch => "prevented",
        CancelDispatch::TransportFailed => "transport_failed",
        CancelDispatch::ServerRejected => "server_rejected",
        CancelDispatch::Unsupported => "unsupported",
    };
    Message::Engine(tablerock_tui::EngineMsg::GridCancelDispatched {
        request_token,
        dispatch: dispatch.into(),
    })
}

fn wall_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn mutation_scope(
    session_id: tablerock_core::SessionId,
    context_revision: u64,
) -> tablerock_core::OperationScope {
    use tablerock_core::{ContextId, IdParts, OperationScope, ProfileId};
    OperationScope::new(
        ProfileId::from_parts(IdParts::new(1, 1).unwrap()).unwrap(),
        session_id,
        ContextId::from_parts(IdParts::new(1, context_revision.max(1)).unwrap()).unwrap(),
    )
}

fn bt_mut(s: &str) -> Result<tablerock_core::BoundedText, String> {
    use tablerock_core::{BoundedText, ByteLimit};
    BoundedText::copy_from_str(s, ByteLimit::new(10_000)).map_err(|_| "text limit".into())
}

fn parse_mut_value(text: &str) -> Result<tablerock_core::OwnedValue, String> {
    use tablerock_core::{OwnedValue, Truncation};
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") {
        return Ok(OwnedValue::null());
    }
    if trimmed.eq_ignore_ascii_case("true") {
        return Ok(OwnedValue::boolean(true));
    }
    if trimmed.eq_ignore_ascii_case("false") {
        return Ok(OwnedValue::boolean(false));
    }
    if let Ok(n) = trimmed.parse::<i64>() {
        return Ok(OwnedValue::signed(n));
    }
    if let Ok(n) = trimmed.parse::<f64>() {
        return Ok(OwnedValue::float64_bits(n.to_bits()));
    }
    let bound = bt_mut(trimmed)?;
    OwnedValue::text(bound, Truncation::Complete).map_err(|_| "invalid text".into())
}

fn mut_fields(
    pairs: &[(String, String)],
) -> Result<Vec<tablerock_core::FieldValue>, String> {
    use tablerock_core::FieldValue;
    pairs
        .iter()
        .map(|(name, value)| Ok(FieldValue::new(bt_mut(name)?, parse_mut_value(value)?)))
        .collect()
}

fn typed_changes_from_specs(
    changes: &[tablerock_tui::effect::MutationChangeSpec],
) -> Result<Vec<tablerock_core::MutationChange>, String> {
    use tablerock_core::MutationChange;
    use tablerock_tui::effect::MutationChangeSpec;
    let mut typed = Vec::new();
    for change in changes {
        typed.push(match change {
            MutationChangeSpec::Insert { values } => MutationChange::InsertRow {
                values: mut_fields(values)?,
            },
            MutationChangeSpec::Update {
                locator,
                assignments,
            } => MutationChange::UpdateRow {
                locator: mut_fields(locator)?,
                assignments: mut_fields(assignments)?,
            },
            MutationChangeSpec::Delete { locator } => MutationChange::DeleteRow {
                locator: mut_fields(locator)?,
            },
        });
    }
    Ok(typed)
}

fn preview_lines_from_plan(plan: &tablerock_core::MutationPlan) -> Vec<String> {
    use tablerock_core::MutationChange;
    plan.changes()
        .iter()
        .enumerate()
        .map(|(i, change)| match change {
            MutationChange::InsertRow { values } => {
                format!(
                    "{i}: INSERT fields={} (typed plan; not executed text)",
                    values.len()
                )
            }
            MutationChange::UpdateRow {
                locator,
                assignments,
            } => format!(
                "{i}: UPDATE set={} where={} (typed plan; not executed text)",
                assignments.len(),
                locator.len()
            ),
            MutationChange::DeleteRow { locator } => {
                format!(
                    "{i}: DELETE where={} (typed plan; not executed text)",
                    locator.len()
                )
            }
            _ => format!("{i}: other change"),
        })
        .collect()
}

/// Register a reviewed plan; returns handle for later apply (consume-once).
async fn review_mutations(
    reviews: Arc<Mutex<tablerock_core::MutationReviewRegistry>>,
    request_token: RequestToken,
    session_id_hex: String,
    context_revision: u64,
    database: String,
    schema: String,
    table: String,
    changes: Vec<tablerock_tui::effect::MutationChangeSpec>,
) -> Message {
    use tablerock_core::{
        IdParts, MutationId, MutationPlan, MutationPlanLimits, MutationTarget, Revision,
        ReviewTokenId, SessionId,
    };

    let session_id = match session_id_hex.parse::<SessionId>() {
        Ok(id) => id,
        Err(_) => {
            return Message::Engine(tablerock_tui::EngineMsg::MutationReviewFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label("invalid session id".into()),
            });
        }
    };
    let typed = match typed_changes_from_specs(&changes) {
        Ok(t) if !t.is_empty() => t,
        Ok(_) => {
            return Message::Engine(tablerock_tui::EngineMsg::MutationReviewFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label("no staged changes".into()),
            });
        }
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::MutationReviewFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(e),
            });
        }
    };
    let limits = match MutationPlanLimits::new(256, 64, 256 * 1024, 1024 * 1024, 60_000) {
        Ok(l) => l,
        Err(_) => {
            return Message::Engine(tablerock_tui::EngineMsg::MutationReviewFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label("invalid mutation limits".into()),
            });
        }
    };
    let scope = mutation_scope(session_id, context_revision);
    let target = MutationTarget::PostgreSqlRelation {
        database: match bt_mut(&database) {
            Ok(v) => v,
            Err(e) => {
                return Message::Engine(tablerock_tui::EngineMsg::MutationReviewFailed {
                    request_token,
                    context_revision,
                    reason: FailureProjection::Label(e),
                });
            }
        },
        schema: match bt_mut(&schema) {
            Ok(v) => v,
            Err(e) => {
                return Message::Engine(tablerock_tui::EngineMsg::MutationReviewFailed {
                    request_token,
                    context_revision,
                    reason: FailureProjection::Label(e),
                });
            }
        },
        relation: match bt_mut(&table) {
            Ok(v) => v,
            Err(e) => {
                return Message::Engine(tablerock_tui::EngineMsg::MutationReviewFailed {
                    request_token,
                    context_revision,
                    reason: FailureProjection::Label(e),
                });
            }
        },
    };
    let plan = match MutationPlan::new(
        MutationId::from_parts(IdParts::new(1, request_token.max(1)).unwrap()).unwrap(),
        scope,
        Revision::INITIAL,
        target,
        typed,
        limits,
    ) {
        Ok(p) => p,
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::MutationReviewFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(e.to_string()),
            });
        }
    };
    let lines = preview_lines_from_plan(&plan);
    let now = wall_ms();
    let issued = now;
    let expires = now.saturating_add(30_000);
    let token_id =
        ReviewTokenId::from_parts(IdParts::new(1, request_token.saturating_add(1).max(2)).unwrap())
            .unwrap();
    let reviewed = match plan.review(token_id, issued, expires) {
        Ok(r) => r,
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::MutationReviewFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(format!("review: {e:?}")),
            });
        }
    };
    {
        let mut reg = reviews.lock().await;
        if let Err(e) = reg.insert(reviewed, now) {
            return Message::Engine(tablerock_tui::EngineMsg::MutationReviewFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(format!("registry insert: {e:?}")),
            });
        }
    }
    Message::Engine(tablerock_tui::EngineMsg::MutationReviewReady {
        request_token,
        context_revision,
        review_token_hex: token_id.to_string(),
        expires_at_ms: expires,
        lines,
    })
}

/// Authorize by handle (consume-once) then apply. Expired/missing → re-review.
async fn apply_mutations(
    sessions: Arc<Mutex<SessionRegistry>>,
    reviews: Arc<Mutex<tablerock_core::MutationReviewRegistry>>,
    request_token: RequestToken,
    session_id_hex: String,
    context_revision: u64,
    review_token_hex: String,
) -> Message {
    use tablerock_core::{Revision, ReviewTokenId, SessionId};
    use tablerock_engine::MutationTransactionState;

    let session_id = match session_id_hex.parse::<SessionId>() {
        Ok(id) => id,
        Err(_) => {
            return Message::Engine(tablerock_tui::EngineMsg::MutationFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label("invalid session id".into()),
                needs_re_review: false,
            });
        }
    };
    let token_id = match review_token_hex.parse::<ReviewTokenId>() {
        Ok(t) => t,
        Err(_) => {
            return Message::Engine(tablerock_tui::EngineMsg::MutationFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label("invalid review token".into()),
                needs_re_review: true,
            });
        }
    };
    let scope = mutation_scope(session_id, context_revision);
    let now = wall_ms();
    let authorized = {
        let mut reg = reviews.lock().await;
        match reg.authorize(token_id, now, scope, Revision::INITIAL) {
            Ok(a) => a,
            Err(e) => {
                let needs = matches!(
                    e,
                    tablerock_core::ReviewRegistryError::TokenNotFound
                        | tablerock_core::ReviewRegistryError::Review(
                            tablerock_core::ReviewError::Expired
                        )
                        | tablerock_core::ReviewRegistryError::Review(
                            tablerock_core::ReviewError::ClockBeforeIssue
                        )
                        | tablerock_core::ReviewRegistryError::Review(
                            tablerock_core::ReviewError::ScopeMismatch
                        )
                        | tablerock_core::ReviewRegistryError::Review(
                            tablerock_core::ReviewError::RevisionMismatch
                        )
                );
                return Message::Engine(tablerock_tui::EngineMsg::MutationFailed {
                    request_token,
                    context_revision,
                    reason: FailureProjection::Label(format!(
                        "authorize failed ({e:?}); re-review required"
                    )),
                    needs_re_review: needs,
                });
            }
        }
    };

    let session = {
        let registry = sessions.lock().await;
        registry.session(session_id)
    };
    let Some(session) = session else {
        return Message::Engine(tablerock_tui::EngineMsg::MutationFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label("session not registered".into()),
            needs_re_review: true,
        });
    };

    match session.apply_authorized_mutation(authorized).await {
        Ok(outcome) => {
            let committed = matches!(outcome.transaction, MutationTransactionState::Committed);
            let detail = format!("{:?}", outcome.transaction);
            Message::Engine(tablerock_tui::EngineMsg::MutationApplied {
                request_token,
                context_revision,
                committed,
                change_count: outcome.changes.len(),
                detail,
            })
        }
        Err(error) => Message::Engine(tablerock_tui::EngineMsg::MutationFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label(error.to_string()),
            needs_re_review: false,
        }),
    }
}

async fn load_foreign_keys(
    sessions: Arc<Mutex<SessionRegistry>>,
    request_token: RequestToken,
    session_id_hex: String,
    context_revision: u64,
    schema: String,
    table: String,
    local_column: String,
    cell_value: String,
) -> Message {
    use tablerock_core::{
        Engine as CoreEngine, IdParts, PageIdentity, PageLimits, ResultId, Revision, StatementText,
    };
    use tablerock_engine::{DriverPageRequest, FilterValue};

    let session_id = match session_id_hex.parse::<SessionId>() {
        Ok(id) => id,
        Err(_) => {
            return Message::Engine(tablerock_tui::EngineMsg::ForeignKeysFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label("invalid session id".into()),
            });
        }
    };
    let session = {
        let registry = sessions.lock().await;
        registry.session(session_id)
    };
    let Some(session) = session else {
        return Message::Engine(tablerock_tui::EngineMsg::ForeignKeysFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label("session not registered".into()),
        });
    };
    if session.engine() != CoreEngine::PostgreSql {
        return Message::Engine(tablerock_tui::EngineMsg::ForeignKeysFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label("foreign keys only for PostgreSQL".into()),
        });
    }
    let sql = "SELECT \
        la.attname::text, \
        fn.nspname::text, \
        fc.relname::text, \
        fa.attname::text \
     FROM pg_catalog.pg_constraint con \
     JOIN pg_catalog.pg_class c ON c.oid = con.conrelid \
     JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
     JOIN pg_catalog.pg_class fc ON fc.oid = con.confrelid \
     JOIN pg_catalog.pg_namespace fn ON fn.oid = fc.relnamespace \
     JOIN LATERAL unnest(con.conkey, con.confkey) \
       WITH ORDINALITY AS u(local_attnum, foreign_attnum, ord) ON true \
     JOIN pg_catalog.pg_attribute la \
       ON la.attrelid = c.oid AND la.attnum = u.local_attnum \
     JOIN pg_catalog.pg_attribute fa \
       ON fa.attrelid = fc.oid AND fa.attnum = u.foreign_attnum \
     WHERE con.contype = 'f' \
       AND n.nspname = $1 \
       AND c.relname = $2 \
       AND la.attname = $3 \
     ORDER BY con.conname, u.ord \
     LIMIT 8";
    let statement = match StatementText::new(sql) {
        Ok(s) => s,
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::ForeignKeysFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(e.to_string()),
            });
        }
    };
    let limits = PageLimits::new(8, 8, 64 * 1024, 4 * 1024);
    let mut stream = match session
        .start_page_stream(DriverPageRequest::PostgreSqlStatement {
            statement,
            parameters: vec![
                FilterValue::Text(schema),
                FilterValue::Text(table),
                FilterValue::Text(local_column),
            ],
            limits,
            max_cell_bytes: 4 * 1024,
        })
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::ForeignKeysFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(e.to_string()),
            });
        }
    };
    let identity = PageIdentity::new(
        ResultId::from_parts(IdParts::new(1, 9_002).unwrap()).unwrap(),
        Revision::INITIAL,
        CoreEngine::PostgreSql,
    );
    let page = match stream.next_page(identity, 0).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return Message::Engine(tablerock_tui::EngineMsg::ForeignKeysFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label("no foreign key on this column".into()),
            });
        }
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::ForeignKeysFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(e.to_string()),
            });
        }
    };
    if page.envelope().row_count() == 0 {
        return Message::Engine(tablerock_tui::EngineMsg::ForeignKeysFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label("no foreign key on this column".into()),
        });
    }
    // Columns: local, foreign_schema, foreign_table, foreign_col
    let text_at = |row: u32, col: u32| -> String {
        page.cell(row, col)
            .map(|c| String::from_utf8_lossy(c.bytes()).into_owned())
            .unwrap_or_default()
    };
    Message::Engine(tablerock_tui::EngineMsg::ForeignKeyEdge {
        request_token,
        context_revision,
        foreign_schema: text_at(0, 1),
        foreign_table: text_at(0, 2),
        foreign_column: text_at(0, 3),
        filter_value: cell_value,
    })
}

async fn load_relation_structure(
    sessions: Arc<Mutex<SessionRegistry>>,
    request_token: RequestToken,
    session_id_hex: String,
    context_revision: u64,
    schema: String,
    table: String,
) -> Message {
    use tablerock_core::{
        Engine as CoreEngine, IdParts, PageIdentity, PageLimits, ResultId, Revision, StatementText,
    };
    use tablerock_engine::{DriverPageRequest, FilterValue};

    let session_id = match session_id_hex.parse::<SessionId>() {
        Ok(id) => id,
        Err(_) => {
            return Message::Engine(tablerock_tui::EngineMsg::RelationStructureFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label("invalid session id".into()),
            });
        }
    };
    let session = {
        let registry = sessions.lock().await;
        registry.session(session_id)
    };
    let Some(session) = session else {
        return Message::Engine(tablerock_tui::EngineMsg::RelationStructureFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label("session not registered".into()),
        });
    };
    let sql = "SELECT a.attname::text, \
            pg_catalog.format_type(a.atttypid, a.atttypmod), \
            CASE WHEN a.attnotnull THEN 'NOT NULL' ELSE 'NULL' END, \
            COALESCE(pg_catalog.pg_get_expr(d.adbin, d.adrelid), '') \
     FROM pg_catalog.pg_attribute a \
     JOIN pg_catalog.pg_class c ON c.oid = a.attrelid \
     JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
     LEFT JOIN pg_catalog.pg_attrdef d \
       ON d.adrelid = a.attrelid AND d.adnum = a.attnum \
     WHERE n.nspname = $1 \
       AND c.relname = $2 \
       AND a.attnum > 0 \
       AND NOT a.attisdropped \
     ORDER BY a.attnum \
     LIMIT 256";
    let statement = match StatementText::new(sql) {
        Ok(s) => s,
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::RelationStructureFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(e.to_string()),
            });
        }
    };
    let limits = PageLimits::new(256, 8, 256 * 1024, 8 * 1024);
    let mut stream = match session
        .start_page_stream(DriverPageRequest::PostgreSqlStatement {
            statement,
            parameters: vec![FilterValue::Text(schema.clone()), FilterValue::Text(table.clone())],
            limits,
            max_cell_bytes: 8 * 1024,
        })
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::RelationStructureFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(e.to_string()),
            });
        }
    };
    let identity = PageIdentity::new(
        ResultId::from_parts(IdParts::new(1, 9_003).unwrap()).unwrap(),
        Revision::INITIAL,
        CoreEngine::PostgreSql,
    );
    let page = match stream.next_page(identity, 0).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return Message::Engine(tablerock_tui::EngineMsg::RelationStructureFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label("relation has no columns".into()),
            });
        }
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::RelationStructureFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(e.to_string()),
            });
        }
    };
    let mut columns = Vec::new();
    for row in 0..page.envelope().row_count() {
        let name = page
            .cell(row, 0)
            .map(|c| String::from_utf8_lossy(c.bytes()).into_owned())
            .unwrap_or_default();
        let ty = page
            .cell(row, 1)
            .map(|c| String::from_utf8_lossy(c.bytes()).into_owned())
            .unwrap_or_default();
        let nulls = page
            .cell(row, 2)
            .map(|c| String::from_utf8_lossy(c.bytes()).into_owned())
            .unwrap_or_default();
        let default = page
            .cell(row, 3)
            .map(|c| String::from_utf8_lossy(c.bytes()).into_owned())
            .unwrap_or_default();
        if default.is_empty() {
            columns.push(format!("{name} {ty} {nulls}"));
        } else {
            columns.push(format!("{name} {ty} {nulls} DEFAULT {default}"));
        }
    }
    Message::Engine(tablerock_tui::EngineMsg::RelationStructure {
        request_token,
        context_revision,
        schema,
        table,
        columns,
    })
}

async fn execute_table_op(
    sessions: Arc<Mutex<SessionRegistry>>,
    request_token: RequestToken,
    session_id_hex: String,
    context_revision: u64,
    op: String,
    schema: String,
    table: String,
    new_table: String,
) -> Message {
    use tablerock_core::{
        Engine as CoreEngine, IdParts, PageIdentity, PageLimits, ResultId, Revision, StatementText,
    };
    use tablerock_engine::{DriverPageRequest, quote_ident};

    let session_id = match session_id_hex.parse::<SessionId>() {
        Ok(id) => id,
        Err(_) => {
            return Message::Engine(tablerock_tui::EngineMsg::TableOpFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label("invalid session id".into()),
            });
        }
    };
    let session = {
        let registry = sessions.lock().await;
        registry.session(session_id)
    };
    let Some(session) = session else {
        return Message::Engine(tablerock_tui::EngineMsg::TableOpFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label("session not registered".into()),
        });
    };
    if session.engine() != CoreEngine::PostgreSql {
        return Message::Engine(tablerock_tui::EngineMsg::TableOpFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label("table ops only for PostgreSQL".into()),
        });
    }
    let (qs, qt) = match (quote_ident(&schema), quote_ident(&table)) {
        (Ok(s), Ok(t)) => (s, t),
        _ => {
            return Message::Engine(tablerock_tui::EngineMsg::TableOpFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label("invalid identifier".into()),
            });
        }
    };
    // Fixed op vocabulary only — never free-form operator SQL.
    let sql = match op.as_str() {
        "truncate" => format!("TRUNCATE TABLE {qs}.{qt}"),
        "drop" => format!("DROP TABLE {qs}.{qt}"),
        "rename" => {
            let qn = match quote_ident(&new_table) {
                Ok(n) => n,
                Err(_) => {
                    return Message::Engine(tablerock_tui::EngineMsg::TableOpFailed {
                        request_token,
                        context_revision,
                        reason: FailureProjection::Label("invalid new table name".into()),
                    });
                }
            };
            format!("ALTER TABLE {qs}.{qt} RENAME TO {qn}")
        }
        other => {
            return Message::Engine(tablerock_tui::EngineMsg::TableOpFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(format!("unknown table op: {other}")),
            });
        }
    };
    let statement = match StatementText::new(&sql) {
        Ok(s) => s,
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::TableOpFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(e.to_string()),
            });
        }
    };
    let limits = PageLimits::new(1, 1, 1024, 256);
    let mut stream = match session
        .start_page_stream(DriverPageRequest::PostgreSqlStatement {
            statement,
            parameters: Vec::new(),
            limits,
            max_cell_bytes: 256,
        })
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::TableOpFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(e.to_string()),
            });
        }
    };
    let identity = PageIdentity::new(
        ResultId::from_parts(IdParts::new(1, 9_004).unwrap()).unwrap(),
        Revision::INITIAL,
        CoreEngine::PostgreSql,
    );
    // Drain: DDL may return empty page or error.
    match stream.next_page(identity, 0).await {
        Ok(_) => Message::Engine(tablerock_tui::EngineMsg::TableOpDone {
            request_token,
            context_revision,
            op,
            schema,
            table,
        }),
        Err(e) => Message::Engine(tablerock_tui::EngineMsg::TableOpFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label(e.to_string()),
        }),
    }
}

async fn load_activity(
    sessions: Arc<Mutex<SessionRegistry>>,
    request_token: RequestToken,
    session_id_hex: String,
    context_revision: u64,
) -> Message {
    use tablerock_core::{
        Engine as CoreEngine, IdParts, PageIdentity, PageLimits, ResultId, Revision, StatementText,
    };
    use tablerock_engine::DriverPageRequest;

    let session_id = match session_id_hex.parse::<SessionId>() {
        Ok(id) => id,
        Err(_) => {
            return Message::Engine(tablerock_tui::EngineMsg::ActivityFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label("invalid session id".into()),
            });
        }
    };
    let session = {
        let registry = sessions.lock().await;
        registry.session(session_id)
    };
    let Some(session) = session else {
        return Message::Engine(tablerock_tui::EngineMsg::ActivityFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label("session not registered".into()),
        });
    };
    // Fixed query only; no cancel/terminate in this checkpoint (permission gates next).
    let sql = "SELECT pid::text, \
            usename::text, \
            application_name::text, \
            state::text, \
            left(query, 80) \
     FROM pg_catalog.pg_stat_activity \
     WHERE backend_type = 'client backend' \
     ORDER BY backend_start DESC NULLS LAST \
     LIMIT 32";
    let statement = match StatementText::new(sql) {
        Ok(s) => s,
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::ActivityFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(e.to_string()),
            });
        }
    };
    let limits = PageLimits::new(32, 8, 256 * 1024, 8 * 1024);
    let mut stream = match session
        .start_page_stream(DriverPageRequest::PostgreSqlStatement {
            statement,
            parameters: Vec::new(),
            limits,
            max_cell_bytes: 8 * 1024,
        })
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::ActivityFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(e.to_string()),
            });
        }
    };
    let identity = PageIdentity::new(
        ResultId::from_parts(IdParts::new(1, 9_005).unwrap()).unwrap(),
        Revision::INITIAL,
        CoreEngine::PostgreSql,
    );
    let page = match stream.next_page(identity, 0).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return Message::Engine(tablerock_tui::EngineMsg::ActivitySnapshot {
                request_token,
                context_revision,
                lines: vec!["(no client backends)".into()],
            });
        }
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::ActivityFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(e.to_string()),
            });
        }
    };
    let mut lines = Vec::new();
    for row in 0..page.envelope().row_count() {
        let mut parts = Vec::new();
        for col in 0..page.envelope().column_count().min(5) {
            let t = page
                .cell(row, col)
                .map(|c| {
                    if c.is_null() {
                        "∅".into()
                    } else {
                        String::from_utf8_lossy(c.bytes()).into_owned()
                    }
                })
                .unwrap_or_default();
            parts.push(t);
        }
        lines.push(parts.join(" · "));
    }
    Message::Engine(tablerock_tui::EngineMsg::ActivitySnapshot {
        request_token,
        context_revision,
        lines,
    })
}

async fn scan_redis_keys(
    sessions: Arc<Mutex<SessionRegistry>>,
    request_token: RequestToken,
    session_id_hex: String,
    context_revision: u64,
    pattern: String,
    count: u32,
) -> Message {
    use tablerock_core::{Engine as CoreEngine, PageLimits};
    use tablerock_engine::{DriverPageRequest, DriverPageStream};

    let session_id = match session_id_hex.parse::<SessionId>() {
        Ok(id) => id,
        Err(_) => {
            return Message::Engine(tablerock_tui::EngineMsg::RedisKeysFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label("invalid session id".into()),
            });
        }
    };
    let session = {
        let registry = sessions.lock().await;
        registry.session(session_id)
    };
    let Some(session) = session else {
        return Message::Engine(tablerock_tui::EngineMsg::RedisKeysFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label("session not registered".into()),
        });
    };
    if session.engine() != CoreEngine::Redis {
        return Message::Engine(tablerock_tui::EngineMsg::RedisKeysFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label("SCAN keys only for Redis".into()),
        });
    }
    let _ = pattern; // pattern applied in a later MATCH filter; first page is full SCAN
    let limits = PageLimits::new(count.max(1), 1, 256 * 1024, 4 * 1024);
    let mut stream = match session
        .start_page_stream(DriverPageRequest::RedisKeyScan {
            limits,
            max_cell_bytes: 4 * 1024,
            scan_count: count.max(1),
            max_scan_rounds: 8,
        })
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::RedisKeysFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(e.to_string()),
            });
        }
    };
    use tablerock_core::{IdParts, PageIdentity, ResultId, Revision};
    let identity = PageIdentity::new(
        ResultId::from_parts(IdParts::new(1, 9_010).unwrap()).unwrap(),
        Revision::INITIAL,
        CoreEngine::Redis,
    );
    let page = match stream.next_page(identity, 0).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return Message::Engine(tablerock_tui::EngineMsg::RedisKeysLoaded {
                request_token,
                context_revision,
                keys: Vec::new(),
                has_more: false,
            });
        }
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::RedisKeysFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(e.to_string()),
            });
        }
    };
    let mut keys = Vec::new();
    for row in 0..page.envelope().row_count() {
        if let Ok(cell) = page.cell(row, 0) {
            if !cell.is_null() {
                keys.push(String::from_utf8_lossy(cell.bytes()).into_owned());
            }
        }
    }
    // Heuristic: full page means more may exist.
    let has_more = page.envelope().row_count() >= count.max(1);
    Message::Engine(tablerock_tui::EngineMsg::RedisKeysLoaded {
        request_token,
        context_revision,
        keys,
        has_more,
    })
}

async fn import_csv_apply(
    sessions: Arc<Mutex<SessionRegistry>>,
    request_token: RequestToken,
    session_id_hex: String,
    context_revision: u64,
    database: String,
    schema: String,
    table: String,
    path: String,
) -> Message {
    use std::fs;

    use tablerock_core::{
        BoundedText, ByteLimit, Engine, IdParts, MutationId, MutationTarget, OperationScope,
        ProfileId, ReviewTokenId, Revision, SessionId as CoreSessionId,
    };

    use crate::import_apply::apply_csv_inserts;
    use crate::import_csv::parse_csv;

    let session_id = match session_id_hex.parse::<SessionId>() {
        Ok(id) => id,
        Err(_) => {
            return Message::Engine(tablerock_tui::EngineMsg::MutationFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label("invalid session id".into()),
                needs_re_review: false,
            });
        }
    };
    let session = {
        let registry = sessions.lock().await;
        registry.session(session_id)
    };
    let Some(session) = session else {
        return Message::Engine(tablerock_tui::EngineMsg::MutationFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label("session not registered".into()),
            needs_re_review: false,
        });
    };

    let csv_text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::MutationFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(format!("read {path}: {e}")),
                needs_re_review: false,
            });
        }
    };
    let table_data = match parse_csv(&csv_text, 10_000, 64 * 1024) {
        Ok(t) => t,
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::MutationFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(e.to_string()),
                needs_re_review: false,
            });
        }
    };

    let bt = |s: &str| {
        BoundedText::copy_from_str(s, ByteLimit::new(256)).unwrap_or_else(|_| {
            BoundedText::copy_from_str("x", ByteLimit::new(1)).expect("tiny")
        })
    };
    let target = match session.engine() {
        Engine::PostgreSql => MutationTarget::PostgreSqlRelation {
            database: bt(if database.is_empty() {
                "postgres"
            } else {
                &database
            }),
            schema: bt(&schema),
            relation: bt(&table),
        },
        Engine::ClickHouse => MutationTarget::ClickHouseTable {
            database: bt(if database.is_empty() {
                "default"
            } else {
                &database
            }),
            table: bt(&table),
        },
        Engine::Redis => {
            return Message::Engine(tablerock_tui::EngineMsg::MutationFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label("CSV import unsupported for Redis".into()),
                needs_re_review: false,
            });
        }
    };

    let scope = OperationScope::new(
        ProfileId::from_parts(IdParts::new(0, request_token.max(1)).expect("id"))
            .expect("profile"),
        // Reuse session id parts when possible; otherwise mint stable opaque IDs.
        CoreSessionId::from_bytes(session_id.to_bytes()).unwrap_or_else(|_| {
            CoreSessionId::from_parts(IdParts::new(0, request_token.max(1) + 1).expect("id"))
                .expect("session")
        }),
        tablerock_core::ContextId::from_parts(
            IdParts::new(0, request_token.max(1) + 2).expect("id"),
        )
        .expect("context"),
    );
    let mutation_id =
        MutationId::from_parts(IdParts::new(0, request_token.max(1) + 3).expect("id"))
            .expect("mutation");
    let token =
        ReviewTokenId::from_parts(IdParts::new(0, request_token.max(1) + 4).expect("id"))
            .expect("token");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(1);

    match apply_csv_inserts(
        session,
        &table_data,
        target,
        scope,
        Revision::from_wire_u64(context_revision),
        mutation_id,
        token,
        64 * 1024,
        256,
        now,
        60_000,
    )
    .await
    {
        Ok(outcome) => {
            let committed = matches!(
                outcome.transaction,
                tablerock_engine::MutationTransactionState::Committed
            );
            Message::Engine(tablerock_tui::EngineMsg::MutationApplied {
                request_token,
                context_revision,
                committed,
                change_count: outcome.changes.len(),
                detail: format!("import csv → {schema}.{table} ({:?})", outcome.transaction),
            })
        }
        Err(e) => Message::Engine(tablerock_tui::EngineMsg::MutationFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label(e.to_string()),
            needs_re_review: false,
        }),
    }
}

async fn export_result(request_token: RequestToken, path: String, body: String) -> Message {
    use crate::file_effects::{AtomicFileWriter, validate_export_path};

    let dest = match validate_export_path(&path) {
        Ok(p) => p,
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::ExportFailed {
                request_token,
                reason: FailureProjection::Label(e.to_string()),
                partial_removed: false,
            });
        }
    };
    let mut writer = match AtomicFileWriter::create(dest.clone()) {
        Ok(w) => w,
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::ExportFailed {
                request_token,
                reason: FailureProjection::Label(e.to_string()),
                partial_removed: false,
            });
        }
    };
    if let Err(e) = writer.write_all(body.as_bytes()) {
        writer.abort();
        return Message::Engine(tablerock_tui::EngineMsg::ExportFailed {
            request_token,
            reason: FailureProjection::Label(e.to_string()),
            partial_removed: true,
        });
    }
    match writer.finish() {
        Ok(bytes) => Message::Engine(tablerock_tui::EngineMsg::ExportDone {
            request_token,
            path: dest.display().to_string(),
            bytes,
        }),
        Err(e) => Message::Engine(tablerock_tui::EngineMsg::ExportFailed {
            request_token,
            reason: FailureProjection::Label(e.to_string()),
            partial_removed: true,
        }),
    }
}

/// Streaming full re-query export: SELECT pages → encoder → atomic file.
async fn export_stream_query(
    sessions: Arc<Mutex<SessionRegistry>>,
    request_token: RequestToken,
    session_id_hex: String,
    context_revision: u64,
    statement: String,
    path: String,
    format: String,
) -> Message {
    use std::sync::atomic::{AtomicBool, Ordering};

    use tablerock_core::{
        BoundedText, ByteLimit, Engine, IdParts, PageIdentity, PageLimits, ResultId, Revision,
        StatementText,
    };
    use tablerock_engine::DriverPageRequest;

    use crate::stream_export::{StreamExportError, StreamExportFormat, StreamExporter};

    let _ = context_revision;
    let session_id = match session_id_hex.parse::<SessionId>() {
        Ok(id) => id,
        Err(_) => {
            return Message::Engine(tablerock_tui::EngineMsg::ExportFailed {
                request_token,
                reason: FailureProjection::Label("invalid session id".into()),
                partial_removed: false,
            });
        }
    };
    let session = {
        let registry = sessions.lock().await;
        registry.session(session_id)
    };
    let Some(session) = session else {
        return Message::Engine(tablerock_tui::EngineMsg::ExportFailed {
            request_token,
            reason: FailureProjection::Label("session not registered".into()),
            partial_removed: false,
        });
    };

    let sql = match StatementText::new(statement) {
        Ok(s) => s,
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::ExportFailed {
                request_token,
                reason: FailureProjection::Label(e.to_string()),
                partial_removed: false,
            });
        }
    };

    let engine = session.engine();
    let limits = PageLimits::new(PAGE_ROWS, 256, 4 * 1024 * 1024, 64 * 1024);
    let request = match engine {
        Engine::PostgreSql => DriverPageRequest::PostgreSqlStatement {
            statement: sql,
            parameters: Vec::new(),
            limits,
            max_cell_bytes: 64 * 1024,
        },
        Engine::ClickHouse => {
            let query_id = BoundedText::copy_from_str(
                &format!("export-{request_token}"),
                ByteLimit::new(128),
            )
            .unwrap_or_else(|_| {
                BoundedText::copy_from_str("export", ByteLimit::new(16)).expect("short")
            });
            DriverPageRequest::ClickHouseStatement {
                statement: sql,
                query_id,
                limits,
                max_cell_bytes: 64 * 1024,
            }
        }
        Engine::Redis => {
            return Message::Engine(tablerock_tui::EngineMsg::ExportFailed {
                request_token,
                reason: FailureProjection::Label(
                    "streaming re-query export unsupported for Redis".into(),
                ),
                partial_removed: false,
            });
        }
    };

    let mut stream = match session.start_page_stream(request).await {
        Ok(s) => s,
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::ExportFailed {
                request_token,
                reason: FailureProjection::Label(e.to_string()),
                partial_removed: false,
            });
        }
    };

    let fmt = StreamExportFormat::parse(&format);
    let mut exporter = match StreamExporter::create(&path, fmt, None) {
        Ok(e) => e,
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::ExportFailed {
                request_token,
                reason: FailureProjection::Label(e.to_string()),
                partial_removed: false,
            });
        }
    };

    let low = request_token.max(1);
    let result_id = ResultId::from_parts(IdParts::new(0, low).expect("nonzero token"))
        .expect("result id");
    let identity = PageIdentity::new(result_id, Revision::INITIAL, engine);
    let mut start_row = 0_u64;
    let cancel = AtomicBool::new(false);
    // Best-effort: if session cancel is requested externally, mid-page loops still finish;
    // Drop of exporter on failure aborts the temp file.
    let _ = &cancel;

    loop {
        if cancel.load(Ordering::SeqCst) {
            exporter.abort();
            return Message::Engine(tablerock_tui::EngineMsg::ExportFailed {
                request_token,
                reason: FailureProjection::Label("export cancelled".into()),
                partial_removed: true,
            });
        }
        match stream.next_page(identity, start_row).await {
            Ok(Some(page)) => {
                let (columns, rows) = page_to_string_table(&page);
                if let Err(e) = exporter.write_page(&columns, &rows) {
                    exporter.abort();
                    return Message::Engine(tablerock_tui::EngineMsg::ExportFailed {
                        request_token,
                        reason: FailureProjection::Label(e.to_string()),
                        partial_removed: true,
                    });
                }
                let count = u64::from(page.envelope().row_count());
                start_row = start_row.saturating_add(count);
                if start_row >= MAX_QUERY_ROWS {
                    break;
                }
                if page.envelope().delivery() == tablerock_core::PageDelivery::Final {
                    break;
                }
            }
            Ok(None) => break,
            Err(e) => {
                exporter.abort();
                return Message::Engine(tablerock_tui::EngineMsg::ExportFailed {
                    request_token,
                    reason: FailureProjection::Label(e.to_string()),
                    partial_removed: true,
                });
            }
        }
    }

    match exporter.finish() {
        Ok(outcome) => Message::Engine(tablerock_tui::EngineMsg::ExportDone {
            request_token,
            path,
            bytes: outcome.bytes,
        }),
        Err(StreamExportError::Cancelled { .. }) => {
            Message::Engine(tablerock_tui::EngineMsg::ExportFailed {
                request_token,
                reason: FailureProjection::Label("export cancelled".into()),
                partial_removed: true,
            })
        }
        Err(e) => Message::Engine(tablerock_tui::EngineMsg::ExportFailed {
            request_token,
            reason: FailureProjection::Label(e.to_string()),
            partial_removed: true,
        }),
    }
}

fn page_to_string_table(page: &tablerock_core::ResultPage) -> (Vec<String>, Vec<Vec<String>>) {
    use tablerock_core::{Truncation, ValueKind};
    let envelope = page.envelope();
    let columns: Vec<String> = page.columns().iter().map(|c| c.name().to_owned()).collect();
    let col_count = envelope.column_count();
    let row_count = envelope.row_count();
    let mut rows = Vec::with_capacity(row_count as usize);
    for row in 0..row_count {
        let mut cells = Vec::with_capacity(col_count as usize);
        for col in 0..col_count {
            let cell = page.cell(row, col).expect("in-range cell");
            let text = if cell.is_null() {
                "NULL".into()
            } else {
                match cell.kind() {
                    ValueKind::Boolean => {
                        if cell.bytes().first() == Some(&1) {
                            "true".into()
                        } else {
                            "false".into()
                        }
                    }
                    ValueKind::Signed => {
                        let mut buf = [0u8; 8];
                        let b = cell.bytes();
                        let n = b.len().min(8);
                        buf[8 - n..].copy_from_slice(&b[..n]);
                        i64::from_be_bytes(buf).to_string()
                    }
                    ValueKind::Unsigned | ValueKind::Float64 => {
                        let mut buf = [0u8; 8];
                        let b = cell.bytes();
                        let n = b.len().min(8);
                        buf[8 - n..].copy_from_slice(&b[..n]);
                        if cell.kind() == ValueKind::Float64 {
                            f64::from_bits(u64::from_be_bytes(buf)).to_string()
                        } else {
                            u64::from_be_bytes(buf).to_string()
                        }
                    }
                    ValueKind::Binary | ValueKind::Unknown | ValueKind::Invalid => {
                        let b = cell.bytes();
                        let take = b.len().min(16);
                        let hex: String = b[..take]
                            .iter()
                            .map(|x| format!("{x:02x}"))
                            .collect::<Vec<_>>()
                            .join(" ");
                        if b.len() > take {
                            format!("{hex} …")
                        } else {
                            hex
                        }
                    }
                    _ => {
                        let mut s = String::from_utf8_lossy(cell.bytes()).into_owned();
                        if matches!(cell.truncation(), Truncation::Truncated { .. }) {
                            s.push('…');
                        }
                        s
                    }
                }
            };
            cells.push(text);
        }
        rows.push(cells);
    }
    (columns, rows)
}

async fn open_redis_key(
    sessions: Arc<Mutex<SessionRegistry>>,
    request_token: RequestToken,
    session_id_hex: String,
    context_revision: u64,
    key: String,
) -> Message {
    let session_id = match session_id_hex.parse::<SessionId>() {
        Ok(id) => id,
        Err(_) => {
            return Message::Engine(tablerock_tui::EngineMsg::RedisKeyViewFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label("invalid session id".into()),
            });
        }
    };
    let session = {
        let registry = sessions.lock().await;
        registry.session(session_id)
    };
    let Some(session) = session else {
        return Message::Engine(tablerock_tui::EngineMsg::RedisKeyViewFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label("session not registered".into()),
        });
    };
    match session.redis_key_view_lines(key.as_bytes()).await {
        Ok((kind_label, lines)) => Message::Engine(tablerock_tui::EngineMsg::RedisKeyViewLoaded {
            request_token,
            context_revision,
            key,
            kind_label,
            lines,
        }),
        Err(e) => Message::Engine(tablerock_tui::EngineMsg::RedisKeyViewFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label(e.to_string()),
        }),
    }
}

async fn load_redis_info(
    sessions: Arc<Mutex<SessionRegistry>>,
    request_token: RequestToken,
    session_id_hex: String,
    context_revision: u64,
) -> Message {
    let session_id = match session_id_hex.parse::<SessionId>() {
        Ok(id) => id,
        Err(_) => {
            return Message::Engine(tablerock_tui::EngineMsg::RedisInfoFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label("invalid session id".into()),
            });
        }
    };
    let session = {
        let registry = sessions.lock().await;
        registry.session(session_id)
    };
    let Some(session) = session else {
        return Message::Engine(tablerock_tui::EngineMsg::RedisInfoFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label("session not registered".into()),
        });
    };
    match session.redis_info_lines().await {
        Ok((sampled_at_ms, lines)) => Message::Engine(tablerock_tui::EngineMsg::RedisInfoLoaded {
            request_token,
            context_revision,
            sampled_at_ms,
            lines,
        }),
        Err(e) => Message::Engine(tablerock_tui::EngineMsg::RedisInfoFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label(e.to_string()),
        }),
    }
}

async fn signal_backend(
    sessions: Arc<Mutex<SessionRegistry>>,
    request_token: RequestToken,
    session_id_hex: String,
    context_revision: u64,
    kind: String,
    pid: i32,
) -> Message {
    use tablerock_core::{
        Engine as CoreEngine, IdParts, PageIdentity, PageLimits, ResultId, Revision, StatementText,
    };
    use tablerock_engine::{DriverPageRequest, FilterValue};

    let session_id = match session_id_hex.parse::<SessionId>() {
        Ok(id) => id,
        Err(_) => {
            return Message::Engine(tablerock_tui::EngineMsg::BackendSignalFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label("invalid session id".into()),
            });
        }
    };
    let session = {
        let registry = sessions.lock().await;
        registry.session(session_id)
    };
    let Some(session) = session else {
        return Message::Engine(tablerock_tui::EngineMsg::BackendSignalFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label("session not registered".into()),
        });
    };
    // Fixed function vocabulary only; pid is a bound integer parameter.
    let sql = match kind.as_str() {
        "cancel" => "SELECT pg_catalog.pg_cancel_backend($1::int4)::text",
        "terminate" => "SELECT pg_catalog.pg_terminate_backend($1::int4)::text",
        other => {
            return Message::Engine(tablerock_tui::EngineMsg::BackendSignalFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(format!("unknown signal: {other}")),
            });
        }
    };
    let statement = match StatementText::new(sql) {
        Ok(s) => s,
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::BackendSignalFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(e.to_string()),
            });
        }
    };
    let limits = PageLimits::new(1, 1, 1024, 64);
    let mut stream = match session
        .start_page_stream(DriverPageRequest::PostgreSqlStatement {
            statement,
            parameters: vec![FilterValue::Integer(i64::from(pid))],
            limits,
            max_cell_bytes: 64,
        })
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return Message::Engine(tablerock_tui::EngineMsg::BackendSignalFailed {
                request_token,
                context_revision,
                reason: FailureProjection::Label(e.to_string()),
            });
        }
    };
    let identity = PageIdentity::new(
        ResultId::from_parts(IdParts::new(1, 9_006).unwrap()).unwrap(),
        Revision::INITIAL,
        CoreEngine::PostgreSql,
    );
    match stream.next_page(identity, 0).await {
        Ok(Some(page)) => {
            let ack = page
                .cell(0, 0)
                .map(|c| String::from_utf8_lossy(c.bytes()) == "t" || String::from_utf8_lossy(c.bytes()) == "true")
                .unwrap_or(false);
            Message::Engine(tablerock_tui::EngineMsg::BackendSignalDone {
                request_token,
                context_revision,
                kind,
                pid,
                acknowledged: ack,
            })
        }
        Ok(None) => Message::Engine(tablerock_tui::EngineMsg::BackendSignalDone {
            request_token,
            context_revision,
            kind,
            pid,
            acknowledged: false,
        }),
        Err(e) => Message::Engine(tablerock_tui::EngineMsg::BackendSignalFailed {
            request_token,
            context_revision,
            reason: FailureProjection::Label(e.to_string()),
        }),
    }
}

/// Load PRIMARY KEY column names via the driver page stream (bound params).
async fn fetch_primary_key_columns(
    session: &dyn tablerock_engine::DriverSession,
    schema: &str,
    table: &str,
) -> Option<Vec<String>> {
    use tablerock_core::{
        Engine as CoreEngine, IdParts, PageIdentity, PageLimits, ResultId, Revision, StatementText,
    };
    use tablerock_engine::{DriverPageRequest, FilterValue};
    let sql = "SELECT a.attname::text \
         FROM pg_catalog.pg_index i \
         JOIN pg_catalog.pg_class c ON c.oid = i.indrelid \
         JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
         JOIN LATERAL unnest(i.indkey) WITH ORDINALITY AS k(attnum, ord) ON true \
         JOIN pg_catalog.pg_attribute a \
           ON a.attrelid = c.oid AND a.attnum = k.attnum AND NOT a.attisdropped \
         WHERE i.indisprimary \
           AND n.nspname = $1 \
           AND c.relname = $2 \
         ORDER BY k.ord";
    let statement = StatementText::new(sql).ok()?;
    let limits = PageLimits::new(32, 8, 64 * 1024, 4 * 1024);
    let mut stream = session
        .start_page_stream(DriverPageRequest::PostgreSqlStatement {
            statement,
            parameters: vec![
                FilterValue::Text(schema.to_owned()),
                FilterValue::Text(table.to_owned()),
            ],
            limits,
            max_cell_bytes: 4 * 1024,
        })
        .await
        .ok()?;
    let identity = PageIdentity::new(
        ResultId::from_parts(IdParts::new(1, 9_001).ok()?).ok()?,
        Revision::INITIAL,
        CoreEngine::PostgreSql,
    );
    let page = stream.next_page(identity, 0).await.ok()??;
    let mut names = Vec::new();
    let rows = page.envelope().row_count();
    for row in 0..rows {
        if let Ok(cell) = page.cell(row, 0) {
            if !cell.is_null() {
                names.push(String::from_utf8_lossy(cell.bytes()).into_owned());
            }
        }
    }
    Some(names)
}

fn project_page_message(
    request_token: RequestToken,
    context_revision: u64,
    page: tablerock_core::ResultPage,
    complete: bool,
    identity_columns: Option<Vec<String>>,
) -> Message {
    use tablerock_core::{RowTotal, Truncation, ValueKind};
    let envelope = page.envelope();
    let columns: Vec<String> = page.columns().iter().map(|c| c.name().to_owned()).collect();
    let col_count = envelope.column_count();
    let row_count = envelope.row_count();
    let mut cells = Vec::with_capacity(row_count as usize * col_count as usize);
    for row in 0..row_count {
        for col in 0..col_count {
            let cell = page.cell(row, col).expect("in-range cell");
            let truncated = matches!(cell.truncation(), Truncation::Truncated { .. });
            let original = match cell.truncation() {
                Truncation::Truncated {
                    original_byte_len: Some(n),
                } => Some(n),
                Truncation::Truncated {
                    original_byte_len: None,
                } => None,
                Truncation::Complete => None,
            };
            let kind_label = match cell.kind() {
                ValueKind::Null => "null",
                ValueKind::Boolean => "boolean",
                ValueKind::Signed
                | ValueKind::Unsigned
                | ValueKind::Float64
                | ValueKind::Decimal => "number",
                ValueKind::Temporal => "temporal",
                ValueKind::Text => "text",
                ValueKind::Structured => "structured",
                ValueKind::Binary => "binary",
                ValueKind::Invalid => "invalid",
                ValueKind::Unknown => "unknown",
            };
            let text = if cell.is_null() {
                String::new()
            } else {
                match cell.kind() {
                    ValueKind::Boolean => {
                        if cell.bytes().first() == Some(&1) {
                            "true".into()
                        } else {
                            "false".into()
                        }
                    }
                    ValueKind::Signed => {
                        let mut buf = [0u8; 8];
                        let b = cell.bytes();
                        let n = b.len().min(8);
                        buf[8 - n..].copy_from_slice(&b[..n]);
                        i64::from_be_bytes(buf).to_string()
                    }
                    ValueKind::Unsigned | ValueKind::Float64 => {
                        let mut buf = [0u8; 8];
                        let b = cell.bytes();
                        let n = b.len().min(8);
                        buf[8 - n..].copy_from_slice(&b[..n]);
                        if cell.kind() == ValueKind::Float64 {
                            f64::from_bits(u64::from_be_bytes(buf)).to_string()
                        } else {
                            u64::from_be_bytes(buf).to_string()
                        }
                    }
                    ValueKind::Binary | ValueKind::Unknown | ValueKind::Invalid => {
                        let b = cell.bytes();
                        let take = b.len().min(16);
                        let hex: String = b[..take]
                            .iter()
                            .map(|x| format!("{x:02x}"))
                            .collect::<Vec<_>>()
                            .join(" ");
                        if b.len() > take {
                            format!("{hex} …")
                        } else {
                            hex
                        }
                    }
                    _ => String::from_utf8_lossy(cell.bytes()).into_owned(),
                }
            };
            let empty = text.is_empty() && !cell.is_null();
            let distinction =
                distinction_from_kind_label(kind_label, cell.is_null(), truncated, empty);
            cells.push(ProjectedCell {
                text,
                distinction,
                byte_len: cell.bytes().len() as u64,
                original_byte_len: original,
            });
        }
    }
    let totals_exact = match envelope.total_rows() {
        RowTotal::Known(n) => Some(n),
        RowTotal::Unknown => None,
    };
    let truncated = cells
        .iter()
        .any(|c| c.distinction == CellDistinction::Truncated);
    Message::Engine(tablerock_tui::EngineMsg::GridPage {
        request_token,
        context_revision,
        start_row: envelope.start_row(),
        columns,
        cells,
        row_count,
        totals_exact,
        totals_estimated: None,
        bytes: envelope.arena_byte_len(),
        truncated,
        complete,
        identity_columns,
    })
}

fn catalog_kind_label(kind: tablerock_core::CatalogNodeKind) -> &'static str {
    use tablerock_core::{
        CatalogNodeKind, ClickHouseObjectKind, PostgreSqlObjectKind, RedisKeyKind,
    };
    match kind {
        CatalogNodeKind::PostgreSqlDatabase | CatalogNodeKind::ClickHouseDatabase => "database",
        CatalogNodeKind::PostgreSqlSchema => "schema",
        CatalogNodeKind::PostgreSqlObject(PostgreSqlObjectKind::Table) => "table",
        CatalogNodeKind::PostgreSqlObject(PostgreSqlObjectKind::View) => "view",
        CatalogNodeKind::PostgreSqlObject(PostgreSqlObjectKind::MaterializedView) => "matview",
        CatalogNodeKind::PostgreSqlObject(PostgreSqlObjectKind::ForeignTable) => "ftable",
        CatalogNodeKind::PostgreSqlObject(PostgreSqlObjectKind::Sequence) => "sequence",
        CatalogNodeKind::PostgreSqlObject(_) => "object",
        CatalogNodeKind::PostgreSqlColumn | CatalogNodeKind::ClickHouseColumn => "column",
        CatalogNodeKind::ClickHouseObject(ClickHouseObjectKind::Table) => "table",
        CatalogNodeKind::ClickHouseObject(ClickHouseObjectKind::View) => "view",
        CatalogNodeKind::ClickHouseObject(ClickHouseObjectKind::Dictionary) => "dict",
        CatalogNodeKind::ClickHouseObject(_) => "object",
        CatalogNodeKind::RedisLogicalDatabase => "db",
        CatalogNodeKind::RedisNamespace => "ns",
        CatalogNodeKind::RedisKey(RedisKeyKind::String) => "string",
        CatalogNodeKind::RedisKey(_) => "key",
    }
}

async fn delete_profile(
    persistence: Arc<Mutex<Option<PersistenceActor>>>,
    request_token: RequestToken,
    profile_id_hex: String,
) -> Message {
    let joined = tokio::task::spawn_blocking(move || {
        let profile_id = profile_id_hex
            .parse::<ProfileId>()
            .map_err(|_| "invalid profile id".to_owned())?;
        let guard = persistence.blocking_lock();
        let Some(actor) = guard.as_ref() else {
            return Err("persistence unavailable".to_owned());
        };
        let Some(aggregate) = actor
            .get_profile(profile_id)
            .map_err(|error| error.to_string())?
        else {
            return Err("profile not found".to_owned());
        };
        let revision = aggregate.connection().revision();
        actor
            .delete_profile(profile_id, revision)
            .map_err(|error| error.to_string())
    })
    .await;
    match joined {
        Ok(Ok(())) => Message::Profiles(ProfilesMsg::Deleted { request_token }),
        Ok(Err(label)) => Message::Profiles(ProfilesMsg::DeleteFailed {
            request_token,
            reason: FailureProjection::Label(label),
        }),
        Err(_) => Message::Profiles(ProfilesMsg::DeleteFailed {
            request_token,
            reason: FailureProjection::Label("task-failed".into()),
        }),
    }
}

async fn delete_group(
    persistence: Arc<Mutex<Option<PersistenceActor>>>,
    request_token: RequestToken,
    group_name: String,
) -> Message {
    let joined = tokio::task::spawn_blocking(move || {
        let guard = persistence.blocking_lock();
        let Some(actor) = guard.as_ref() else {
            return Err("persistence unavailable".to_owned());
        };
        actor
            .delete_group(&group_name)
            .map_err(|error| error.to_string())
            .map(|_| ())
    })
    .await;
    match joined {
        Ok(Ok(())) => Message::Profiles(ProfilesMsg::Deleted { request_token }),
        Ok(Err(label)) => Message::Profiles(ProfilesMsg::DeleteFailed {
            request_token,
            reason: FailureProjection::Label(label),
        }),
        Err(_) => Message::Profiles(ProfilesMsg::DeleteFailed {
            request_token,
            reason: FailureProjection::Label("task-failed".into()),
        }),
    }
}

async fn connect_profile(
    persistence: Arc<Mutex<Option<PersistenceActor>>>,
    sessions: Arc<Mutex<SessionRegistry>>,
    request_token: RequestToken,
    profile_id_hex: String,
    override_password: Option<String>,
) -> Message {
    let draft =
        match load_profile_draft(persistence, profile_id_hex.clone(), override_password).await {
            Ok(draft) => draft,
            Err(label) if label == "password prompt required" => {
                return Message::Engine(tablerock_tui::EngineMsg::PasswordPromptRequired {
                    request_token,
                    profile_id_hex,
                });
            }
            Err(label) => {
                return Message::Engine(tablerock_tui::EngineMsg::ConnectFailed {
                    request_token,
                    reason: FailureProjection::Label(label),
                });
            }
        };
    connect_session(
        sessions,
        request_token,
        draft,
        false,
        Some(profile_id_hex),
    )
    .await
}

async fn reconnect_session(
    sessions: Arc<Mutex<SessionRegistry>>,
    request_token: RequestToken,
    draft: ConnectionDraft,
    attempt: u32,
) -> Message {
    use tablerock_tui::{next_backoff_ms, stop_on_failure_label};
    if next_backoff_ms(attempt).is_none() {
        return Message::Engine(tablerock_tui::EngineMsg::ReconnectStopped {
            request_token,
            reason: FailureProjection::Label("reconnect budget exhausted".into()),
        });
    }
    // Delay is declarative (next_backoff_ms); executor may sleep before re-dispatch.
    // This attempt connects immediately so auth-stop never burns wall-clock in tests.
    match open_described_session(draft.clone()).await {
        Ok((session, identity, _, tunnel)) => {
            let session_id = match mint_session_id() {
                Ok(id) => id,
                Err(label) => {
                    let _ = session.shutdown().await;
                    drop(tunnel);
                    return Message::Engine(tablerock_tui::EngineMsg::ReconnectStopped {
                        request_token,
                        reason: FailureProjection::Label(label),
                    });
                }
            };
            let mut registry = sessions.lock().await;
            match registry.register_with_tunnel(session_id, session, tunnel) {
                Ok(_) => Message::Engine(tablerock_tui::EngineMsg::ConnectOk {
                    request_token,
                    session_id_hex: session_id.to_string(),
                    identity,
                    temporary: true,
                    engine_label: match draft.engine {
                        EngineKind::PostgreSql => "PostgreSQL",
                        EngineKind::ClickHouse => "ClickHouse",
                        EngineKind::Redis => "Redis",
                    }
                    .into(),
                    profile_id_hex: None,
                }),
                Err(error) => Message::Engine(tablerock_tui::EngineMsg::ReconnectStopped {
                    request_token,
                    reason: FailureProjection::Label(error.to_string()),
                }),
            }
        }
        Err(label) if stop_on_failure_label(&label) => {
            Message::Engine(tablerock_tui::EngineMsg::ReconnectStopped {
                request_token,
                reason: FailureProjection::Label(label),
            })
        }
        Err(_label) => match next_backoff_ms(attempt.saturating_add(1)) {
            Some(next_delay_ms) => Message::Engine(tablerock_tui::EngineMsg::Reconnecting {
                request_token,
                attempt: attempt.saturating_add(1),
                next_delay_ms,
            }),
            None => Message::Engine(tablerock_tui::EngineMsg::ReconnectStopped {
                request_token,
                reason: FailureProjection::Label("reconnect budget exhausted".into()),
            }),
        },
    }
}

async fn load_profile_draft(
    persistence: Arc<Mutex<Option<PersistenceActor>>>,
    profile_id_hex: String,
    override_password: Option<String>,
) -> Result<ConnectionDraft, String> {
    tokio::task::spawn_blocking(move || {
        let profile_id = profile_id_hex
            .parse::<ProfileId>()
            .map_err(|_| "invalid profile id".to_owned())?;
        let guard = persistence.blocking_lock();
        let Some(actor) = guard.as_ref() else {
            return Err("persistence unavailable".to_owned());
        };
        let Some(aggregate) = actor
            .get_profile(profile_id)
            .map_err(|error| error.to_string())?
        else {
            return Err("profile not found".to_owned());
        };
        let mut draft = aggregate_to_draft(&aggregate)?;
        if let Some(password) = override_password {
            draft.password = password;
        }
        Ok(draft)
    })
    .await
    .map_err(|_| "task-failed".to_owned())?
}

fn aggregate_to_draft(aggregate: &ProfileAggregate) -> Result<ConnectionDraft, String> {
    use tablerock_core::ProfileProperty;
    use tablerock_engine::{SecretPromptPort, SecretResolutionError, resolve_for_connect};

    struct FailPrompt;
    impl SecretPromptPort for FailPrompt {
        fn request(
            &mut self,
            _field: tablerock_core::SecretField,
            _profile: &tablerock_core::ProfileName,
        ) -> Result<tablerock_engine::ResolvedSecret, SecretResolutionError> {
            Err(SecretResolutionError::PromptFailed)
        }
    }

    let connection = aggregate.connection();
    let props = connection.properties();
    let literal = |property: ProfileProperty| -> Option<String> {
        props
            .binding(property)
            .and_then(|binding| binding.literal_value().map(str::to_owned))
    };
    let mut prompt = FailPrompt;
    let mut password = String::new();
    if let Some(binding) = props.binding(ProfileProperty::Password) {
        match resolve_for_connect(binding, connection.name(), &mut prompt) {
            Ok(Some(secret)) => {
                password = String::from_utf8_lossy(secret.as_bytes()).into_owned();
            }
            Ok(None) => {}
            Err(SecretResolutionError::PromptFailed) => {
                return Err("password prompt required".into());
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    let engine = match connection.engine() {
        Engine::PostgreSql => EngineKind::PostgreSql,
        Engine::ClickHouse => EngineKind::ClickHouse,
        Engine::Redis => EngineKind::Redis,
    };
    let tls_mode = match connection.tls_policy() {
        TlsPolicy::Disabled => TlsModeSpec::Off,
        TlsPolicy::VerifySystemRoots => TlsModeSpec::VerifyCa,
        TlsPolicy::VerifyCustomCa | TlsPolicy::DangerousAcceptInvalidCertificate(_) => {
            TlsModeSpec::VerifyFull
        }
    };
    let mut ssh_password = String::new();
    if let Some(binding) = props.binding(ProfileProperty::SshPassword) {
        match resolve_for_connect(binding, connection.name(), &mut prompt) {
            Ok(Some(secret)) => {
                ssh_password = String::from_utf8_lossy(secret.as_bytes()).into_owned();
            }
            Ok(None) => {}
            Err(SecretResolutionError::PromptFailed) => {
                return Err("SSH password prompt required".into());
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    let mut ssh_private_key = String::new();
    if let Some(binding) = props.binding(ProfileProperty::SshPrivateKey) {
        match resolve_for_connect(binding, connection.name(), &mut prompt) {
            Ok(Some(secret)) => {
                ssh_private_key = String::from_utf8_lossy(secret.as_bytes()).into_owned();
            }
            Ok(None) => {}
            Err(SecretResolutionError::PromptFailed) => {
                return Err("SSH private key prompt required".into());
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(ConnectionDraft {
        engine,
        name: connection.name().as_str().to_owned(),
        group: aggregate
            .organization()
            .group()
            .map(|group| group.as_str().to_owned())
            .unwrap_or_default(),
        environment: String::new(),
        host: literal(ProfileProperty::Host).unwrap_or_default(),
        port: literal(ProfileProperty::Port).unwrap_or_default(),
        database: literal(ProfileProperty::DefaultContext).unwrap_or_default(),
        username: literal(ProfileProperty::Username).unwrap_or_default(),
        password,
        password_source: PasswordSourceSpec::DangerousPlaintext,
        tls_mode,
        plaintext_acknowledged: true,
        ssh_host: literal(ProfileProperty::SshHost).unwrap_or_default(),
        ssh_port: literal(ProfileProperty::SshPort).unwrap_or_else(|| "22".to_owned()),
        ssh_username: literal(ProfileProperty::SshUsername).unwrap_or_default(),
        ssh_password,
        ssh_private_key,
        ssh_known_hosts_path: literal(ProfileProperty::SshKnownHostsPath).unwrap_or_default(),
    })
}

/// Connect + describe. Caller owns shutdown/register.
///
/// When `draft.ssh_host` is set, opens a fail-closed known_hosts bastion tunnel
/// and rewrites the driver endpoint to `127.0.0.1:local_port`. The tunnel is
/// returned so the caller can keep it alive with the session.
async fn open_described_session(
    draft: ConnectionDraft,
) -> Result<
    (
        Box<dyn DriverSession>,
        String,
        u64,
        Option<tablerock_engine::LocalForwardTunnel>,
    ),
    String,
> {
    use tablerock_engine::{
        ClickHouseCompression, ClickHouseConnectConfig, ClickHouseSession, ClickHouseTlsMode,
        LocalForwardTunnel, PostgresConnectConfig, PostgresSession, PostgresTlsMode,
        RedisConnectConfig, RedisConnectionSecurity, RedisCredentials, RedisProtocol, RedisSession,
        RedisTlsMode, SshAuthMaterial, SshHostKeyPolicy, SshPasswordAuth, SshPublicKeyAuth,
        SshTunnelConfig, open_local_forward_tunnel,
    };
    let mut host = draft.host.clone();
    let mut port: u16 = draft.port.parse().map_err(|_| "invalid port".to_owned())?;
    let mut tunnel = None;
    if !draft.ssh_host.trim().is_empty() {
        if draft.ssh_known_hosts_path.trim().is_empty() {
            return Err("SSH known_hosts path required for tunnel connect".to_owned());
        }
        let bastion_port: u16 = if draft.ssh_port.trim().is_empty() {
            22
        } else {
            draft.ssh_port
                .parse()
                .map_err(|_| "invalid SSH port".to_owned())?
        };
        let username = if draft.ssh_username.is_empty() {
            "root".to_owned()
        } else {
            draft.ssh_username.clone()
        };
        let auth = if !draft.ssh_private_key.trim().is_empty() {
            SshAuthMaterial::PublicKey(
                SshPublicKeyAuth::from_openssh_private_key(&username, &draft.ssh_private_key)
                    .map_err(|e| e.to_string())?,
            )
        } else if !draft.ssh_password.is_empty() {
            SshAuthMaterial::Password(SshPasswordAuth::new(username, draft.ssh_password.clone()))
        } else {
            return Err("SSH password or private key required".to_owned());
        };
        let config = SshTunnelConfig {
            bastion_host: draft.ssh_host.clone(),
            bastion_port,
            auth,
            host_key_policy: SshHostKeyPolicy::KnownHostsPath(
                std::path::PathBuf::from(draft.ssh_known_hosts_path.trim()),
            ),
        };
        let opened = open_local_forward_tunnel(&config, host.as_str(), port)
            .await
            .map_err(|e| e.to_string())?;
        host = LocalForwardTunnel::local_host().to_owned();
        port = opened.local_port();
        tunnel = Some(opened);
    }
    let text = |value: &str| {
        tablerock_core::BoundedText::copy_from_str(value, tablerock_core::ByteLimit::new(128))
            .map_err(|e| e.to_string())
    };
    let pg_tls = match draft.tls_mode {
        TlsModeSpec::Off => PostgresTlsMode::Disabled,
        TlsModeSpec::VerifyCa | TlsModeSpec::VerifyFull => PostgresTlsMode::Required,
    };
    let ch_tls = match draft.tls_mode {
        TlsModeSpec::Off => ClickHouseTlsMode::Disable,
        TlsModeSpec::VerifyCa | TlsModeSpec::VerifyFull => ClickHouseTlsMode::Require,
    };
    let redis_tls = match draft.tls_mode {
        TlsModeSpec::Off => RedisTlsMode::Disable,
        TlsModeSpec::VerifyCa | TlsModeSpec::VerifyFull => RedisTlsMode::Require,
    };
    match draft.engine {
        EngineKind::PostgreSql => {
            let session = PostgresSession::connect(&PostgresConnectConfig::new(
                text(&host)?,
                port,
                text(if draft.database.is_empty() {
                    "postgres"
                } else {
                    &draft.database
                })?,
                text(if draft.username.is_empty() {
                    "postgres"
                } else {
                    &draft.username
                })?,
                pg_tls,
            ))
            .await
            .map_err(|e| e.to_string())?;
            let described = session.describe().await.map_err(|e| e.to_string())?;
            Ok((
                Box::new(session) as Box<dyn DriverSession>,
                described.identity().to_owned(),
                described.elapsed_millis(),
                tunnel,
            ))
        }
        EngineKind::ClickHouse => {
            let _ = &draft.password;
            let session = ClickHouseSession::connect(&ClickHouseConnectConfig::new(
                text(&host)?,
                port,
                text(if draft.database.is_empty() {
                    "default"
                } else {
                    &draft.database
                })?,
                text(if draft.username.is_empty() {
                    "default"
                } else {
                    &draft.username
                })?,
                ch_tls,
                ClickHouseCompression::None,
            ));
            let described = session.describe().await.map_err(|e| e.to_string())?;
            Ok((
                Box::new(session) as Box<dyn DriverSession>,
                described.identity().to_owned(),
                described.elapsed_millis(),
                tunnel,
            ))
        }
        EngineKind::Redis => {
            let mut security = RedisConnectionSecurity::new();
            if !draft.password.is_empty() || !draft.username.is_empty() {
                let username = if draft.username.is_empty() {
                    None
                } else {
                    Some(draft.username.as_str())
                };
                security = security
                    .with_credentials(RedisCredentials::new(username, draft.password.as_str()));
            }
            let session = RedisSession::connect(
                &RedisConnectConfig::new(
                    text(&host)?,
                    port,
                    draft.database.parse().unwrap_or(0),
                    RedisProtocol::Resp3,
                    redis_tls,
                ),
                security,
            )
            .await
            .map_err(|e| e.to_string())?;
            let described = session.describe().await.map_err(|e| e.to_string())?;
            Ok((
                Box::new(session) as Box<dyn DriverSession>,
                described.identity().to_owned(),
                described.elapsed_millis(),
                tunnel,
            ))
        }
    }
}

async fn save_connection(
    persistence: Arc<Mutex<Option<PersistenceActor>>>,
    request_token: RequestToken,
    draft: ConnectionDraft,
) -> Message {
    let joined = tokio::task::spawn_blocking(move || {
        let aggregate = draft_to_aggregate(&draft).map_err(|label| label)?;
        let guard = persistence.blocking_lock();
        let Some(actor) = guard.as_ref() else {
            return Err("persistence unavailable".to_owned());
        };
        let token = aggregate
            .persistable()
            .ok_or_else(|| "temporary profile cannot be saved".to_owned())?;
        actor
            .create_profile(token)
            .map_err(|error| error.to_string())
    })
    .await;
    match joined {
        Ok(Ok(())) => Message::Profiles(ProfilesMsg::Saved { request_token }),
        Ok(Err(label)) => Message::Profiles(ProfilesMsg::SaveFailed {
            request_token,
            reason: FailureProjection::Label(label),
        }),
        Err(_) => Message::Profiles(ProfilesMsg::SaveFailed {
            request_token,
            reason: FailureProjection::Label("task-failed".into()),
        }),
    }
}

fn draft_to_aggregate(draft: &ConnectionDraft) -> Result<ProfileAggregate, String> {
    let text = |value: &str| {
        BoundedText::copy_from_str(value, ByteLimit::new(128)).map_err(|error| error.to_string())
    };
    let engine = match draft.engine {
        EngineKind::PostgreSql => Engine::PostgreSql,
        EngineKind::ClickHouse => Engine::ClickHouse,
        EngineKind::Redis => Engine::Redis,
    };
    let mut bindings = vec![
        ProfilePropertyBinding::literal(ProfileProperty::Host, text(&draft.host)?)
            .map_err(|error| error.to_string())?,
        ProfilePropertyBinding::literal(ProfileProperty::Port, text(&draft.port)?)
            .map_err(|error| error.to_string())?,
    ];
    if !draft.database.trim().is_empty() {
        bindings.push(
            ProfilePropertyBinding::literal(
                ProfileProperty::DefaultContext,
                text(&draft.database)?,
            )
            .map_err(|error| error.to_string())?,
        );
    }
    if !draft.username.trim().is_empty() {
        bindings.push(
            ProfilePropertyBinding::literal(ProfileProperty::Username, text(&draft.username)?)
                .map_err(|error| error.to_string())?,
        );
    }
    if !draft.ssh_host.trim().is_empty() {
        let ssh_path_text = |value: &str| {
            BoundedText::copy_from_str(value, ByteLimit::new(4_096)).map_err(|e| e.to_string())
        };
        bindings.push(
            ProfilePropertyBinding::literal(ProfileProperty::SshHost, text(&draft.ssh_host)?)
                .map_err(|error| error.to_string())?,
        );
        let ssh_port = if draft.ssh_port.trim().is_empty() {
            "22"
        } else {
            draft.ssh_port.trim()
        };
        bindings.push(
            ProfilePropertyBinding::literal(ProfileProperty::SshPort, text(ssh_port)?)
                .map_err(|error| error.to_string())?,
        );
        if !draft.ssh_username.trim().is_empty() {
            bindings.push(
                ProfilePropertyBinding::literal(
                    ProfileProperty::SshUsername,
                    text(&draft.ssh_username)?,
                )
                .map_err(|error| error.to_string())?,
            );
        }
        if !draft.ssh_known_hosts_path.trim().is_empty() {
            bindings.push(
                ProfilePropertyBinding::literal(
                    ProfileProperty::SshKnownHostsPath,
                    ssh_path_text(draft.ssh_known_hosts_path.trim())?,
                )
                .map_err(|error| error.to_string())?,
            );
        }
        if !draft.ssh_password.is_empty() {
            bindings.push(ProfilePropertyBinding::secret(
                ProfileProperty::SshPassword,
                SecretSource::new(SecretSourceKind::DangerousPlaintext(
                    DangerousPlaintext::new(
                        draft.ssh_password.as_bytes().to_vec(),
                        PlaintextAcknowledgement::LocalTestingOnly,
                    )
                    .map_err(|error| error.to_string())?,
                )),
            ));
        }
        if !draft.ssh_private_key.trim().is_empty() {
            bindings.push(ProfilePropertyBinding::secret(
                ProfileProperty::SshPrivateKey,
                SecretSource::new(SecretSourceKind::DangerousPlaintext(
                    DangerousPlaintext::new(
                        draft.ssh_private_key.as_bytes().to_vec(),
                        PlaintextAcknowledgement::LocalTestingOnly,
                    )
                    .map_err(|error| error.to_string())?,
                )),
            ));
        }
    }
    let password_source = match draft.password_source {
        PasswordSourceSpec::PromptOnConnect => SecretSourceKind::PromptOnConnect,
        PasswordSourceSpec::DangerousPlaintext => {
            if !draft.plaintext_acknowledged {
                return Err("plaintext password not acknowledged".into());
            }
            SecretSourceKind::DangerousPlaintext(
                DangerousPlaintext::new(
                    draft.password.as_bytes().to_vec(),
                    PlaintextAcknowledgement::LocalTestingOnly,
                )
                .map_err(|error| error.to_string())?,
            )
        }
    };
    bindings.push(ProfilePropertyBinding::secret(
        ProfileProperty::Password,
        SecretSource::new(password_source),
    ));
    let properties = ProfilePropertySet::new(bindings).map_err(|error| error.to_string())?;
    let tls = match draft.tls_mode {
        TlsModeSpec::Off => TlsPolicy::Disabled,
        TlsModeSpec::VerifyCa => TlsPolicy::VerifySystemRoots,
        TlsModeSpec::VerifyFull => TlsPolicy::VerifyCustomCa,
    };
    let low = NEXT_PROFILE_LOW.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let id = ProfileId::from_parts(IdParts::new(1, low).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let connection = ProfileConnectionSnapshot::new(
        ProfileIdentity::new(
            id,
            Revision::INITIAL,
            engine,
            ProfileName::new(text(draft.name.trim())?).map_err(|e| e.to_string())?,
        ),
        properties,
        ProfilePolicy::new(
            tls,
            ProfileSafetyMode::ConfirmWrites,
            ProfileLimits::new(10_000, 30_000, 5_000, 16 * 1024 * 1024)
                .map_err(|e| e.to_string())?,
        ),
    )
    .map_err(|e| e.to_string())?;
    let group = if draft.group.trim().is_empty() {
        None
    } else {
        Some(ProfileGroupName::new(text(draft.group.trim())?).map_err(|e| e.to_string())?)
    };
    let environment = parse_environment(&draft.environment)?;
    let organization = ProfileOrganization::new(group, Vec::new(), false, 0, environment)
        .map_err(|e| e.to_string())?;
    ProfileAggregate::new(
        connection,
        ProfileDurability::Saved,
        organization,
        ProfilePreferences::new(ReconnectPreference::BoundedAutomatic, true, 250)
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

fn parse_environment(raw: &str) -> Result<Option<EnvironmentTag>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    Ok(Some(match raw.to_ascii_lowercase().as_str() {
        "production" | "prod" => EnvironmentTag::Production,
        "staging" => EnvironmentTag::Staging,
        "development" | "dev" => EnvironmentTag::Development,
        "testing" | "test" => EnvironmentTag::Testing,
        other => EnvironmentTag::Custom(
            ProfileTag::new(
                BoundedText::copy_from_str(other, ByteLimit::new(64)).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?,
        ),
    }))
}

fn default_persistence_path() -> PathBuf {
    let mut path = dirs_next_home();
    path.push(".tablerock");
    // Process-local file until cross-process ownership is productized
    // (PathLease is single-process; concurrent PTY tests need isolation).
    path.push(format!("state-{}.db", std::process::id()));
    path
}

fn dirs_next_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}
