//! Async effect executor: pure TUI effects → persistence/engine → messages.

use std::{path::PathBuf, sync::Arc};

use tablerock_core::{
    BoundedText, ByteLimit, DangerousPlaintext, Engine, EnvironmentTag, IdParts,
    PlaintextAcknowledgement, ProfileAggregate, ProfileConnectionSnapshot, ProfileDurability,
    ProfileGroupName, ProfileId, ProfileIdentity, ProfileLimits, ProfileListFilter,
    ProfileListRequest, ProfileName, ProfileOrganization, ProfilePolicy, ProfilePreferences,
    ProfileProperty, ProfilePropertyBinding, ProfilePropertySet, ProfileSafetyMode, ProfileTag,
    ReconnectPreference, Revision, SecretSource, SecretSourceKind, SessionId, TlsPolicy,
};
use tablerock_engine::{CatalogRequest, DriverSession, SessionRegistry};
use tablerock_persistence::PersistenceActor;
use tablerock_tui::{
    CatalogLevelSpec, CatalogNodeProjection, CatalogNodeStatus, ConnectionDraft, Effect,
    EngineKind, FailureProjection, Message, PasswordSourceSpec, ProfilesMsg, RequestToken,
    TlsModeSpec,
};
use tokio::sync::Mutex;

use crate::{RootMessageSender, projection};

static NEXT_PROFILE_LOW: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
static NEXT_SESSION_LOW: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Owns process-local handles used by effect tasks.
pub struct EffectExecutor {
    persistence: Arc<Mutex<Option<PersistenceActor>>>,
    sessions: Arc<Mutex<SessionRegistry>>,
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
                    let message = connect_session(sessions, request_token, draft, temporary).await;
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
        }
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
    connect_session(sessions, request_token, draft, false).await
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
