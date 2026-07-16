//! Deterministic root update path.

use crate::{ActionId, Effect, FocusRegion, Message, Model, Screen, ShellTarget};

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
            if model.size() == (width, height) {
                Update::unchanged()
            } else {
                model.resize(width, height);
                Update::render()
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
