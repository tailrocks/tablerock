//! Deterministic root update path.

use termrock::runtime::UpdateResult;

use crate::{ActionId, Effect, FocusRegion, Message, Model, Screen, ShellTarget};

pub fn update(model: &mut Model, message: Message) -> UpdateResult<Effect> {
    match message {
        Message::Resize { width, height } => {
            if model.size() == (width, height) {
                UpdateResult::clean()
            } else {
                model.resize(width, height);
                UpdateResult::redraw()
            }
        }
        Message::TerminalFocusChanged(focused) => {
            if model.terminal_focused() == focused {
                UpdateResult::clean()
            } else {
                model.set_terminal_focused(focused);
                UpdateResult::redraw()
            }
        }
        Message::Paste(_) => UpdateResult::clean(),
        Message::PointerHovered(target) | Message::PointerDragged(target) => {
            if model.hovered() == target {
                UpdateResult::clean()
            } else {
                model.set_hovered(target);
                UpdateResult::redraw()
            }
        }
        Message::PointerPressed(target) => {
            model.set_pressed(target);
            if let Some(target) = target {
                focus_target(model, target);
            }
            UpdateResult::redraw()
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
            UpdateResult::redraw()
        }
        Message::PointerScrolled { target, .. } => {
            if let Some(target) = target {
                focus_target(model, target);
                UpdateResult::redraw()
            } else {
                UpdateResult::clean()
            }
        }
        Message::EngineResyncRequired => {
            if model.engine_resync_required() {
                UpdateResult::clean()
            } else {
                model.set_engine_resync_required(true);
                UpdateResult::redraw()
            }
        }
        Message::EngineResynchronized => {
            if model.engine_resync_required() {
                model.set_engine_resync_required(false);
                UpdateResult::redraw()
            } else {
                UpdateResult::clean()
            }
        }
        Message::FocusNext => {
            model.set_focus(model.focus().next());
            UpdateResult::redraw()
        }
        Message::FocusPrevious => {
            model.set_focus(model.focus().previous());
            UpdateResult::redraw()
        }
        Message::ActionNext | Message::ActionPrevious if model.focus() == FocusRegion::Actions => {
            let action = match model.selected_action() {
                ActionId::Open => ActionId::Quit,
                ActionId::Quit => ActionId::Open,
            };
            model.set_action(action);
            UpdateResult::redraw()
        }
        Message::ActionNext | Message::ActionPrevious => UpdateResult::clean(),
        Message::Activate if model.focus() == FocusRegion::Actions => {
            activate_selected_action(model)
        }
        Message::Activate => UpdateResult::clean(),
        Message::RequestRedraw => UpdateResult::redraw(),
        Message::Quit => UpdateResult::with_effect(Effect::Exit),
    }
}

fn focus_target(model: &mut Model, target: ShellTarget) {
    match target {
        ShellTarget::Focus(focus) => model.set_focus(focus),
        ShellTarget::Action(action) => {
            model.set_focus(FocusRegion::Actions);
            model.set_action(action);
        }
    }
}

fn activate_selected_action(model: &mut Model) -> UpdateResult<Effect> {
    match model.selected_action() {
        ActionId::Open => {
            model.set_screen(Screen::ConnectionPicker);
            UpdateResult::redraw()
        }
        ActionId::Quit => UpdateResult::with_effect(Effect::Exit),
    }
}
