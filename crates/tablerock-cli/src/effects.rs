//! Async effect executor: pure TUI effects → persistence/engine → messages.

use std::{path::PathBuf, sync::Arc};

use tablerock_core::{
    BoundedText, ByteLimit, DangerousPlaintext, Engine, EnvironmentTag, IdParts,
    PlaintextAcknowledgement, ProfileAggregate, ProfileConnectionSnapshot, ProfileDurability,
    ProfileGroupName, ProfileId, ProfileIdentity, ProfileLimits, ProfileListFilter,
    ProfileListRequest, ProfileName, ProfileOrganization, ProfilePolicy, ProfilePreferences,
    ProfileProperty, ProfilePropertyBinding, ProfilePropertySet, ProfileSafetyMode, ProfileTag,
    ReconnectPreference, Revision, SecretSource, SecretSourceKind, TlsPolicy,
};
use tablerock_persistence::PersistenceActor;
use tablerock_tui::{
    ConnectionDraft, Effect, EngineKind, FailureProjection, Message, PasswordSourceSpec,
    ProfilesMsg, RequestToken, TlsModeSpec,
};
use tokio::sync::Mutex;

use crate::{RootMessageSender, projection};

static NEXT_PROFILE_LOW: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Owns process-local handles used by effect tasks.
pub struct EffectExecutor {
    persistence: Arc<Mutex<Option<PersistenceActor>>>,
    ingress: RootMessageSender,
}

impl EffectExecutor {
    #[must_use]
    pub fn new(persistence: PersistenceActor, ingress: RootMessageSender) -> Self {
        Self {
            persistence: Arc::new(Mutex::new(Some(persistence))),
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
    use tablerock_engine::{
        ClickHouseCompression, ClickHouseConnectConfig, ClickHouseSession, ClickHouseTlsMode,
        DriverSession, PostgresConnectConfig, PostgresSession, PostgresTlsMode, RedisConnectConfig,
        RedisConnectionSecurity, RedisCredentials, RedisProtocol, RedisSession, RedisTlsMode,
    };
    let host = draft.host.clone();
    let port: u16 = match draft.port.parse() {
        Ok(port) => port,
        Err(_) => {
            return Message::Engine(tablerock_tui::EngineMsg::TestFailed {
                request_token,
                reason: FailureProjection::Label("invalid port".into()),
            });
        }
    };
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
    let result = async {
        match draft.engine {
            EngineKind::PostgreSql => {
                // Password is not yet on PostgresConnectConfig; trust/peer fixtures work
                // without it. Wire authenticated connect when engine grows a secret bag.
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
                let _ = session.shutdown().await;
                Ok((described.identity().to_owned(), described.elapsed_millis()))
            }
            EngineKind::ClickHouse => {
                // ClickHouse connect is lazy; describe_server performs the round-trip.
                // Password wiring waits on connect config secret bag (engine gap).
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
                let _ = Box::new(session).shutdown().await;
                Ok((described.identity().to_owned(), described.elapsed_millis()))
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
                let _ = Box::new(session).shutdown().await;
                Ok((described.identity().to_owned(), described.elapsed_millis()))
            }
        }
    }
    .await;
    match result {
        Ok((identity, elapsed_millis)) => Message::Engine(tablerock_tui::EngineMsg::TestOk {
            request_token,
            identity,
            elapsed_millis,
        }),
        Err(label) => Message::Engine(tablerock_tui::EngineMsg::TestFailed {
            request_token,
            reason: FailureProjection::Label(label),
        }),
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
