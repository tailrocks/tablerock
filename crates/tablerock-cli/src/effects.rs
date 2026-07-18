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
        Ok((session, identity, elapsed_millis)) => {
            let _ = session.shutdown().await;
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
        Ok((session, identity, _elapsed)) => {
            let session_id = match mint_session_id() {
                Ok(id) => id,
                Err(label) => {
                    let _ = session.shutdown().await;
                    return Message::Engine(tablerock_tui::EngineMsg::ConnectFailed {
                        request_token,
                        reason: FailureProjection::Label(label),
                    });
                }
            };
            let mut registry = sessions.lock().await;
            match registry.register(session_id, session) {
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
    execute_sql(
        sessions,
        results,
        ingress,
        request_token,
        session_id_hex,
        context_revision,
        rendered.sql,
        rendered.parameters,
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
    project_page_message(request_token, context_revision, page, false)
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
    let _ = dispatch;
    Message::Engine(tablerock_tui::EngineMsg::GridCancelDispatched { request_token })
}

fn project_page_message(
    request_token: RequestToken,
    context_revision: u64,
    page: tablerock_core::ResultPage,
    complete: bool,
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
        Ok((session, identity, _)) => {
            let session_id = match mint_session_id() {
                Ok(id) => id,
                Err(label) => {
                    let _ = session.shutdown().await;
                    return Message::Engine(tablerock_tui::EngineMsg::ReconnectStopped {
                        request_token,
                        reason: FailureProjection::Label(label),
                    });
                }
            };
            let mut registry = sessions.lock().await;
            match registry.register(session_id, session) {
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
    })
}

/// Connect + describe. Caller owns shutdown/register.
async fn open_described_session(
    draft: ConnectionDraft,
) -> Result<(Box<dyn DriverSession>, String, u64), String> {
    use tablerock_engine::{
        ClickHouseCompression, ClickHouseConnectConfig, ClickHouseSession, ClickHouseTlsMode,
        PostgresConnectConfig, PostgresSession, PostgresTlsMode, RedisConnectConfig,
        RedisConnectionSecurity, RedisCredentials, RedisProtocol, RedisSession, RedisTlsMode,
    };
    let host = draft.host.clone();
    let port: u16 = draft.port.parse().map_err(|_| "invalid port".to_owned())?;
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
