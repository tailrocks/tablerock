//! Deterministic root update path.

use termrock::runtime::UpdateResult;

use crate::{ActionId, Effect, FocusRegion, Message, Model, Screen};

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
        Message::Activate
            if model.focus() == FocusRegion::Actions
                && model.selected_action() == ActionId::Quit =>
        {
            UpdateResult::with_effect(Effect::Exit)
        }
        Message::Activate if model.focus() == FocusRegion::Actions => {
            model.set_screen(Screen::ConnectionPicker);
            UpdateResult::redraw()
        }
        Message::Activate => UpdateResult::clean(),
        Message::RequestRedraw => UpdateResult::redraw(),
        Message::Quit => UpdateResult::with_effect(Effect::Exit),
    }
}
