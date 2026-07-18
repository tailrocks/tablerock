//! Async effect executor: pure TUI effects → persistence/engine → messages.

use std::{path::PathBuf, sync::Arc};

use tablerock_core::{ProfileListFilter, ProfileListRequest};
use tablerock_persistence::PersistenceActor;
use tablerock_tui::{Effect, FailureProjection, Message, ProfilesMsg, RequestToken};
use tokio::sync::Mutex;

use crate::{RootMessageSender, projection};

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
