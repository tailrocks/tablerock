use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use tablerock_tui::Message;

/// Translate backend-specific input into root semantic intent.
#[must_use]
pub fn map_event(event: Event) -> Option<Message> {
    match event {
        Event::Resize(width, height) => Some(Message::Resize { width, height }),
        Event::FocusGained | Event::FocusLost => Some(Message::RequestRedraw),
        Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
            KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
                Some(Message::FocusPrevious)
            }
            KeyCode::Tab => Some(Message::FocusNext),
            KeyCode::BackTab => Some(Message::FocusPrevious),
            KeyCode::Left => Some(Message::ActionPrevious),
            KeyCode::Right => Some(Message::ActionNext),
            KeyCode::Enter => Some(Message::Activate),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Message::Quit)
            }
            _ => None,
        },
        Event::Key(_) | Event::Mouse(_) | Event::Paste(_) => None,
    }
}
