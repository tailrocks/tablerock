//! Deterministic root update path.

use crate::{
    ActionId, Effect, FocusRegion, Message, Model, Screen, ShellTarget,
    effect::ProfileListFilterSpec,
    message::{EngineMsg, ProfilesMsg},
    model::profiles::ProfileListState,
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
            model.set_profiles(ProfileListState::Loaded {
                request_token,
                rows: items,
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
        Message::Engine(EngineMsg::HealthOk { .. } | EngineMsg::HealthFailed { .. }) => {
            // Health UI lands with plan 006; accept for token plumbing only.
            Update::unchanged()
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
            let action = match model.selected_action() {
                ActionId::Open => ActionId::Quit,
                ActionId::Quit => ActionId::Open,
            };
            model.set_action(action);
            Update::render()
        }
        Message::ActionNext | Message::ActionPrevious => Update::unchanged(),
        Message::Activate if model.focus() == Some(FocusRegion::Actions) => {
            activate_selected_action(model)
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
        ActionId::Open => {
            model.set_screen(Screen::ConnectionPicker);
            Update::render()
        }
        ActionId::Quit => Update::with_effect(Effect::Exit),
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
}
