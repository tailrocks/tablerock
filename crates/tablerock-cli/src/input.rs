use tablerock_tui::{
    Message, PasteText, ScrollDirection, ShellGeometry, ShellKeyAction, default_keymap,
};
use termrock::input::{Event, KeyEventKind, MouseButton, MouseEventKind};
use termrock::keymap::{KeyChord, Keymap};

#[derive(Debug, Clone, Default)]
pub struct InputAdapter {
    geometry: ShellGeometry,
}

impl InputAdapter {
    pub fn set_geometry(&mut self, geometry: ShellGeometry) {
        self.geometry = geometry;
    }

    /// Translate backend input into root semantic intent using painted geometry.
    #[must_use]
    pub fn map_event(&self, event: Event) -> Option<Message> {
        self.map_event_with_keymap(event, &default_keymap())
    }

    #[must_use]
    pub fn map_event_with_keymap(
        &self,
        event: Event,
        keymap: &Keymap<ShellKeyAction>,
    ) -> Option<Message> {
        match event {
            Event::Resize { width, height } => Some(Message::Resize { width, height }),
            Event::FocusGained => Some(Message::TerminalFocusChanged(true)),
            Event::FocusLost => Some(Message::TerminalFocusChanged(false)),
            Event::Paste(text) => Some(Message::Paste(PasteText::bounded(text))),
            Event::Mouse(mouse) => {
                let target = self.geometry.target_at(mouse.position.x, mouse.position.y);
                match mouse.kind {
                    MouseEventKind::Moved => Some(Message::PointerHovered(target)),
                    MouseEventKind::Down(MouseButton::Left) => {
                        Some(Message::PointerPressed(target))
                    }
                    MouseEventKind::Drag(MouseButton::Left) => {
                        Some(Message::PointerDragged(target))
                    }
                    MouseEventKind::Up(MouseButton::Left) => Some(Message::PointerReleased(target)),
                    MouseEventKind::ScrollUp => Some(Message::PointerScrolled {
                        target,
                        direction: ScrollDirection::Up,
                    }),
                    MouseEventKind::ScrollDown => Some(Message::PointerScrolled {
                        target,
                        direction: ScrollDirection::Down,
                    }),
                    MouseEventKind::ScrollLeft => Some(Message::PointerScrolled {
                        target,
                        direction: ScrollDirection::Left,
                    }),
                    MouseEventKind::ScrollRight => Some(Message::PointerScrolled {
                        target,
                        direction: ScrollDirection::Right,
                    }),
                    MouseEventKind::Down(_) | MouseEventKind::Up(_) | MouseEventKind::Drag(_) => {
                        None
                    }
                    _ => None,
                }
            }
            Event::Key(key) if key.kind != KeyEventKind::Release => keymap
                .dispatch(KeyChord::from(key))
                .map(message_for_key_action),
            Event::Key(_) | Event::Unknown => None,
            _ => None,
        }
    }

    /// Convert the selected backend into TermRock's neutral event vocabulary.
    #[must_use]
    pub fn map_backend_event(&self, event: crossterm::event::Event) -> Option<Message> {
        self.map_event(event.into())
    }

    #[must_use]
    pub fn map_backend_event_with_keymap(
        &self,
        event: crossterm::event::Event,
        keymap: &Keymap<ShellKeyAction>,
    ) -> Option<Message> {
        self.map_event_with_keymap(event.into(), keymap)
    }
}

const fn message_for_key_action(action: ShellKeyAction) -> Message {
    match action {
        ShellKeyAction::FocusNext => Message::FocusNext,
        ShellKeyAction::FocusPrevious => Message::FocusPrevious,
        ShellKeyAction::ActionPrevious => Message::ActionPrevious,
        ShellKeyAction::ActionNext => Message::ActionNext,
        ShellKeyAction::Activate => Message::Activate,
        ShellKeyAction::Quit => Message::Quit,
    }
}

/// Translate input that does not require painted geometry.
#[must_use]
pub fn map_event(event: Event) -> Option<Message> {
    InputAdapter::default().map_event(event)
}

/// Convert Crossterm input at the CLI boundary, then route neutral input.
#[must_use]
pub fn map_backend_event(event: crossterm::event::Event) -> Option<Message> {
    InputAdapter::default().map_backend_event(event)
}
