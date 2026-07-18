//! Deterministic root update path.

use crate::{
    ActionId, Effect, FocusRegion, Message, Model, Screen, ShellTarget,
    effect::ProfileListFilterSpec,
    message::{EngineMsg, ProfilesMsg},
    model::{
        SessionFacts,
        profiles::{FailureProjection, ProfileListState},
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    render: bool,
    effect: Option<Effect>,
}

impl Update {
    const fn unchanged() -> Self {
        Self {
            render: false,
            effect: None,
        }
    }

    const fn render() -> Self {
        Self {
            render: true,
            effect: None,
        }
    }

    const fn with_effect(effect: Effect) -> Self {
        Self {
            render: false,
            effect: Some(effect),
        }
    }

    #[must_use]
    pub const fn needs_render(&self) -> bool {
        self.render
    }

    pub fn effects(&self) -> impl Iterator<Item = &Effect> {
        self.effect.iter()
    }
}

pub fn update(model: &mut Model, message: Message) -> Update {
    match message {
        Message::Resize { width, height } => {
            let size_changed = model.size() != (width, height);
            if size_changed {
                model.resize(width, height);
            }
            let bootstrap = maybe_bootstrap_profiles(model);
            if bootstrap.effect.is_some() {
                return Update {
                    render: true,
                    effect: bootstrap.effect,
                };
            }
            if size_changed {
                Update::render()
            } else {
                Update::unchanged()
            }
        }
        Message::FrameRendered(geometry) => {
            if model.reconcile_focus_frame(&geometry) {
                Update::render()
            } else {
                Update::unchanged()
            }
        }
        Message::TerminalFocusChanged(focused) => {
            if model.terminal_focused() == focused {
                Update::unchanged()
            } else {
                model.set_terminal_focused(focused);
                Update::render()
            }
        }
        Message::Paste(text) if model.screen() == Screen::Editor => {
            apply_editor_text(model, text.text());
            Update::render()
        }
        Message::Paste(text)
            if model.screen() == Screen::Connections
                && model.focus() == Some(FocusRegion::Content) =>
        {
            model.profiles_mut().push_search(text.text());
            Update::render()
        }
        Message::Paste(_) => Update::unchanged(),
        Message::PointerHovered(target) | Message::PointerDragged(target) => {
            if model.hovered() == target {
                Update::unchanged()
            } else {
                model.set_hovered(target);
                Update::render()
            }
        }
        Message::PointerPressed(target) => {
            model.set_pressed(target);
            if let Some(target) = target {
                focus_target(model, target);
            }
            Update::render()
        }
        Message::PointerReleased(target) => {
            let pressed = model.pressed();
            model.set_pressed(None);
            model.set_hovered(target);
            if pressed == target
                && let Some(ShellTarget::Action(action)) = target
            {
                model.set_action(action);
                return activate_selected_action(model);
            }
            Update::render()
        }
        Message::PointerScrolled { target, .. } => {
            if let Some(target) = target {
                focus_target(model, target);
                Update::render()
            } else {
                Update::unchanged()
            }
        }
        Message::EngineResyncRequired => {
            if model.engine_resync_required() {
                Update::unchanged()
            } else {
                model.set_engine_resync_required(true);
                Update::render()
            }
        }
        Message::EngineResynchronized => {
            if model.engine_resync_required() {
                model.set_engine_resync_required(false);
                Update::render()
            } else {
                Update::unchanged()
            }
        }
        Message::Profiles(ProfilesMsg::ListLoaded {
            request_token,
            items,
        }) => {
            if model.profiles().active_token() != Some(request_token) {
                return Update::unchanged();
            }
            let selected_id = items.first().map(|row| row.id_hex.clone());
            model.set_profiles(ProfileListState::Loaded {
                request_token,
                rows: items,
                selected_id,
                search: String::new(),
                collapsed: Vec::new(),
            });
            Update::render()
        }
        Message::Profiles(ProfilesMsg::ListFailed {
            request_token,
            reason,
        }) => {
            if model.profiles().active_token() != Some(request_token) {
                return Update::unchanged();
            }
            model.set_profiles(ProfileListState::Failed {
                request_token,
                reason,
            });
            Update::render()
        }
        Message::Profiles(ProfilesMsg::Saved { request_token }) => {
            if model.profiles().active_token() != Some(request_token) {
                return Update::unchanged();
            }
            model.set_screen(Screen::Connections);
            model.set_action(ActionId::Open);
            // Reload list under a new token.
            let token = model.mint_request_token();
            model.set_profiles(ProfileListState::Loading {
                request_token: token,
            });
            Update {
                render: true,
                effect: Some(Effect::LoadProfileList {
                    request_token: token,
                    filter: ProfileListFilterSpec::default(),
                }),
            }
        }
        Message::Profiles(ProfilesMsg::SaveFailed {
            request_token,
            reason,
        }) => {
            if model.profiles().active_token() != Some(request_token) {
                return Update::unchanged();
            }
            model.editor_mut().validation_error = Some(match reason {
                FailureProjection::Label(label) => label,
            });
            Update::render()
        }
        Message::Engine(EngineMsg::HealthOk { .. } | EngineMsg::HealthFailed { .. }) => {
            Update::unchanged()
        }
        Message::Engine(EngineMsg::TestOk {
            identity,
            elapsed_millis,
            ..
        }) => {
            model.editor_mut().test_status = Some(format!("ok: {identity} ({elapsed_millis} ms)"));
            Update::render()
        }
        Message::Engine(EngineMsg::TestFailed { reason, .. }) => {
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            model.editor_mut().test_status = Some(format!("failed: {label}"));
            Update::render()
        }
        Message::Engine(EngineMsg::ConnectOk {
            session_id_hex,
            identity,
            temporary,
            engine_label,
            ..
        }) => {
            model.set_session(Some(SessionFacts {
                session_id_hex,
                identity,
                temporary,
                engine_label,
                status: Some("connected".into()),
            }));
            model.set_screen(Screen::Workbench);
            model.set_action(ActionId::Disconnect);
            Update::render()
        }
        Message::Engine(EngineMsg::ConnectFailed { reason, .. }) => {
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            model.editor_mut().test_status = Some(format!("connect failed: {label}"));
            if let Some(session) = model.session().cloned() {
                let mut session = session;
                session.status = Some(format!("failed: {label}"));
                model.set_session(Some(session));
            }
            Update::render()
        }
        Message::Engine(EngineMsg::DisconnectOk { .. }) => {
            model.set_session(None);
            model.set_screen(Screen::Connections);
            model.set_action(ActionId::Open);
            Update::render()
        }
        Message::Engine(EngineMsg::DisconnectFailed { reason, .. }) => {
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            if let Some(mut session) = model.session().cloned() {
                session.status = Some(format!("disconnect failed: {label}"));
                model.set_session(Some(session));
            }
            Update::render()
        }
        Message::FocusNext => {
            if model.move_focus(false) {
                Update::render()
            } else {
                Update::unchanged()
            }
        }
        Message::FocusPrevious => {
            if model.move_focus(true) {
                Update::render()
            } else {
                Update::unchanged()
            }
        }
        Message::ActionNext | Message::ActionPrevious
            if model.focus() == Some(FocusRegion::Actions) =>
        {
            let reverse = matches!(message, Message::ActionPrevious);
            model.set_action(cycle_action(
                model.screen(),
                model.selected_action(),
                reverse,
            ));
            Update::render()
        }
        Message::ActionNext | Message::ActionPrevious => Update::unchanged(),
        Message::Activate if model.focus() == Some(FocusRegion::Actions) => {
            activate_selected_action(model)
        }
        Message::Activate
            if model.screen() == Screen::Connections
                && model.focus() == Some(FocusRegion::Content) =>
        {
            model.profiles_mut().select_next();
            Update::render()
        }
        Message::Activate if model.screen() == Screen::Editor => {
            model.editor_mut().focus_next();
            Update::render()
        }
        Message::Activate => Update::unchanged(),
        Message::RequestRedraw => Update::render(),
        Message::Quit => Update::with_effect(Effect::Exit),
    }
}

fn focus_target(model: &mut Model, target: ShellTarget) {
    match target {
        ShellTarget::Focus(focus) => {
            let _ = model.request_focus(focus);
        }
        ShellTarget::Action(action) => {
            let _ = model.request_focus(FocusRegion::Actions);
            model.set_action(action);
        }
    }
}

fn activate_selected_action(model: &mut Model) -> Update {
    match model.selected_action() {
        ActionId::Open if model.screen() == Screen::Connections => {
            let Some(profile_id_hex) = model
                .profiles()
                .selected_row()
                .map(|row| row.id_hex.clone())
            else {
                return Update::unchanged();
            };
            let token = model.mint_request_token();
            Update {
                render: true,
                effect: Some(Effect::ConnectProfile {
                    request_token: token,
                    profile_id_hex,
                }),
            }
        }
        ActionId::Open => {
            model.set_screen(Screen::ConnectionPicker);
            Update::render()
        }
        ActionId::New => {
            model.reset_editor();
            model.set_screen(Screen::Editor);
            model.set_action(ActionId::Save);
            Update::render()
        }
        ActionId::Save if model.screen() == Screen::Editor => {
            if !model.editor_mut().validate() {
                return Update::render();
            }
            let token = model.mint_request_token();
            model.set_profiles(ProfileListState::Loading {
                request_token: token,
            });
            // Persist via CLI; reload list after save completes (reuse Load token).
            Update {
                render: true,
                effect: Some(Effect::SaveConnection {
                    request_token: token,
                    draft: connection_draft_from_editor(model.editor()),
                }),
            }
        }
        ActionId::Test if model.screen() == Screen::Editor => {
            if !model.editor_mut().validate() {
                return Update::render();
            }
            let token = model.mint_request_token();
            model.editor_mut().test_status = Some("testing…".into());
            Update {
                render: true,
                effect: Some(Effect::TestConnection {
                    request_token: token,
                    draft: connection_draft_from_editor(model.editor()),
                }),
            }
        }
        ActionId::Connect if model.screen() == Screen::Editor => {
            if !model.editor_mut().validate() {
                return Update::render();
            }
            let token = model.mint_request_token();
            model.editor_mut().test_status = Some("connecting…".into());
            Update {
                render: true,
                effect: Some(Effect::ConnectSession {
                    request_token: token,
                    draft: connection_draft_from_editor(model.editor()),
                    // Editor Connect is temporary until list-row Open saves first.
                    temporary: true,
                }),
            }
        }
        ActionId::Disconnect if model.screen() == Screen::Workbench => {
            let Some(session_id_hex) = model.session().map(|s| s.session_id_hex.clone()) else {
                return Update::unchanged();
            };
            let token = model.mint_request_token();
            if let Some(mut facts) = model.session().cloned() {
                facts.status = Some("disconnecting…".into());
                model.set_session(Some(facts));
            }
            Update {
                render: true,
                effect: Some(Effect::DisconnectSession {
                    request_token: token,
                    session_id_hex,
                }),
            }
        }
        ActionId::Cancel if model.screen() == Screen::Editor => {
            model.set_screen(Screen::Connections);
            model.set_action(ActionId::Open);
            Update::render()
        }
        ActionId::Quit => Update::with_effect(Effect::Exit),
        ActionId::Save
        | ActionId::Test
        | ActionId::Connect
        | ActionId::Disconnect
        | ActionId::Cancel => Update::unchanged(),
    }
}

fn cycle_action(screen: Screen, current: ActionId, reverse: bool) -> ActionId {
    let actions: &[ActionId] = match screen {
        Screen::Editor => &[
            ActionId::Save,
            ActionId::Test,
            ActionId::Connect,
            ActionId::Cancel,
            ActionId::Quit,
        ],
        Screen::Workbench => &[ActionId::Disconnect, ActionId::Quit],
        Screen::Connections | Screen::ConnectionPicker => {
            &[ActionId::Open, ActionId::New, ActionId::Quit]
        }
    };
    let idx = actions.iter().position(|a| *a == current).unwrap_or(0);
    if reverse {
        actions[(idx + actions.len() - 1) % actions.len()]
    } else {
        actions[(idx + 1) % actions.len()]
    }
}

fn connection_draft_from_editor(
    editor: &crate::model::editor::ConnectionFormModel,
) -> crate::effect::ConnectionDraft {
    use crate::effect::{ConnectionDraft, PasswordSourceSpec, TlsModeSpec};
    use crate::model::editor::{PasswordSourceChoice, TlsModeChoice};
    ConnectionDraft {
        engine: editor.engine,
        name: editor.name.clone(),
        group: editor.group.clone(),
        environment: editor.environment.clone(),
        host: editor.host.clone(),
        port: editor.port.clone(),
        database: editor.database.clone(),
        username: editor.username.clone(),
        password: editor.password.clone(),
        password_source: match editor.password_source {
            PasswordSourceChoice::PromptOnConnect => PasswordSourceSpec::PromptOnConnect,
            PasswordSourceChoice::DangerousPlaintext => PasswordSourceSpec::DangerousPlaintext,
        },
        tls_mode: match editor.tls_mode {
            TlsModeChoice::Off => TlsModeSpec::Off,
            TlsModeChoice::VerifyCa => TlsModeSpec::VerifyCa,
            TlsModeChoice::VerifyFull => TlsModeSpec::VerifyFull,
        },
        plaintext_acknowledged: editor.plaintext_acknowledged,
    }
}

fn apply_editor_text(model: &mut Model, text: &str) {
    use crate::model::editor::EditorField;
    let editor = model.editor_mut();
    match editor.focused {
        EditorField::Engine => editor.cycle_engine(),
        EditorField::Name => editor.name.push_str(text),
        EditorField::Group => editor.group.push_str(text),
        EditorField::Environment => editor.environment.push_str(text),
        EditorField::Host => editor.host.push_str(text),
        EditorField::Port => editor.port.push_str(text),
        EditorField::Database => editor.database.push_str(text),
        EditorField::Username => editor.username.push_str(text),
        EditorField::Password => editor.password.push_str(text),
        EditorField::PasswordSource => {
            editor.password_source = match editor.password_source {
                crate::model::editor::PasswordSourceChoice::PromptOnConnect => {
                    crate::model::editor::PasswordSourceChoice::DangerousPlaintext
                }
                crate::model::editor::PasswordSourceChoice::DangerousPlaintext => {
                    crate::model::editor::PasswordSourceChoice::PromptOnConnect
                }
            };
        }
        EditorField::TlsMode => {
            editor.tls_mode = match editor.tls_mode {
                crate::model::editor::TlsModeChoice::Off => {
                    crate::model::editor::TlsModeChoice::VerifyCa
                }
                crate::model::editor::TlsModeChoice::VerifyCa => {
                    crate::model::editor::TlsModeChoice::VerifyFull
                }
                crate::model::editor::TlsModeChoice::VerifyFull => {
                    crate::model::editor::TlsModeChoice::Off
                }
            };
        }
    }
}

fn maybe_bootstrap_profiles(model: &mut Model) -> Update {
    if model.bootstrapped() || model.screen() != Screen::Connections {
        return Update::unchanged();
    }
    model.set_bootstrapped(true);
    let token = model.mint_request_token();
    model.set_profiles(ProfileListState::Loading {
        request_token: token,
    });
    Update::with_effect(Effect::LoadProfileList {
        request_token: token,
        filter: ProfileListFilterSpec::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::profiles::{FailureProjection, ProfileRowProjection};

    #[test]
    fn bootstrap_emits_load_and_rejects_stale_tokens() {
        let mut model = Model::default();
        let first = update(
            &mut model,
            Message::Resize {
                width: 80,
                height: 24,
            },
        );
        assert!(matches!(
            first.effects().next(),
            Some(Effect::LoadProfileList {
                request_token: 1,
                ..
            })
        ));
        assert!(matches!(
            model.profiles(),
            ProfileListState::Loading { request_token: 1 }
        ));

        // Stale completion ignored.
        let stale = update(
            &mut model,
            Message::Profiles(ProfilesMsg::ListLoaded {
                request_token: 99,
                items: vec![],
            }),
        );
        assert!(!stale.needs_render());
        assert!(matches!(
            model.profiles(),
            ProfileListState::Loading { request_token: 1 }
        ));

        let ok = update(
            &mut model,
            Message::Profiles(ProfilesMsg::ListLoaded {
                request_token: 1,
                items: vec![ProfileRowProjection {
                    id_hex: "1".into(),
                    name: "a".into(),
                    engine_label: "PostgreSQL".into(),
                    group: None,
                    favorite: false,
                    target_summary: "localhost:5432".into(),
                    environment: None,
                    production_warning: false,
                    safety_label: "Read only".into(),
                    plaintext_secret_warning: false,
                    live_state: crate::model::profiles::LiveConnectionState::Disconnected,
                }],
            }),
        );
        assert!(ok.needs_render());
        assert_eq!(model.profiles().status_line(), "Profiles: 1");
    }

    #[test]
    fn failure_label_is_redacted_status_only() {
        let mut model = Model::default();
        let _ = update(
            &mut model,
            Message::Resize {
                width: 80,
                height: 24,
            },
        );
        let _ = update(
            &mut model,
            Message::Profiles(ProfilesMsg::ListFailed {
                request_token: 1,
                reason: FailureProjection::Label("unavailable".into()),
            }),
        );
        assert_eq!(
            model.profiles().status_line(),
            "Profiles: error (unavailable)"
        );
    }

    #[test]
    fn test_action_emits_effect_and_records_outcome() {
        let mut model = Model::default();
        model.set_screen(Screen::Editor);
        model.set_action(ActionId::Test);
        model.editor_mut().name = "local".into();
        model.editor_mut().host = "127.0.0.1".into();
        model.editor_mut().port = "5432".into();
        // Focus Actions so Activate would work; call action path via ActionId match.
        for _ in 0..4 {
            let _ = update(&mut model, Message::FocusNext);
        }
        // selected action may have moved; set Test again after focus moves to Actions
        model.set_action(ActionId::Test);
        let result = update(&mut model, Message::Activate);
        assert!(matches!(
            result.effects().next(),
            Some(Effect::TestConnection {
                request_token: 1,
                ..
            })
        ));
        assert_eq!(model.editor().test_status.as_deref(), Some("testing…"));

        let ok = update(
            &mut model,
            Message::Engine(EngineMsg::TestOk {
                request_token: 1,
                identity: "PostgreSQL 17".into(),
                elapsed_millis: 12,
            }),
        );
        assert!(ok.needs_render());
        assert_eq!(
            model.editor().test_status.as_deref(),
            Some("ok: PostgreSQL 17 (12 ms)")
        );

        let fail = update(
            &mut model,
            Message::Engine(EngineMsg::TestFailed {
                request_token: 1,
                reason: FailureProjection::Label("connect".into()),
            }),
        );
        assert!(fail.needs_render());
        assert_eq!(
            model.editor().test_status.as_deref(),
            Some("failed: connect")
        );
    }

    #[test]
    fn temporary_connect_effect_sets_temporary_flag() {
        let mut model = Model::default();
        model.set_screen(Screen::Editor);
        model.set_action(ActionId::Connect);
        model.editor_mut().name = "tmp".into();
        model.editor_mut().host = "127.0.0.1".into();
        model.editor_mut().port = "5432".into();
        for _ in 0..4 {
            let _ = update(&mut model, Message::FocusNext);
        }
        model.set_action(ActionId::Connect);
        let result = update(&mut model, Message::Activate);
        assert!(matches!(
            result.effects().next(),
            Some(Effect::ConnectSession {
                temporary: true,
                ..
            })
        ));
    }

    #[test]
    fn connect_opens_workbench_and_disconnect_returns() {
        let mut model = Model::default();
        model.set_screen(Screen::Editor);
        let _ = update(
            &mut model,
            Message::Engine(EngineMsg::ConnectOk {
                request_token: 1,
                session_id_hex: "00000000000000010000000000000001".into(),
                identity: "PostgreSQL 17".into(),
                temporary: true,
                engine_label: "PostgreSQL".into(),
            }),
        );
        assert_eq!(model.screen(), Screen::Workbench);
        assert!(model.session().is_some());
        assert_eq!(model.selected_action(), ActionId::Disconnect);

        let _ = update(
            &mut model,
            Message::Engine(EngineMsg::DisconnectOk {
                request_token: 2,
                session_id_hex: "00000000000000010000000000000001".into(),
            }),
        );
        assert_eq!(model.screen(), Screen::Connections);
        assert!(model.session().is_none());
    }
}
