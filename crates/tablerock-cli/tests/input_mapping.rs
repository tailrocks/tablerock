use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use tablerock_cli::map_event;
use tablerock_tui::Message;

#[test]
fn maps_only_semantic_shell_intents() {
    for (event, expected) in [
        (
            Event::Resize(80, 24),
            Some(Message::Resize {
                width: 80,
                height: 24,
            }),
        ),
        (
            key(KeyCode::Tab, KeyModifiers::NONE, KeyEventKind::Press),
            Some(Message::FocusNext),
        ),
        (
            key(KeyCode::BackTab, KeyModifiers::SHIFT, KeyEventKind::Press),
            Some(Message::FocusPrevious),
        ),
        (
            key(KeyCode::Left, KeyModifiers::NONE, KeyEventKind::Repeat),
            Some(Message::ActionPrevious),
        ),
        (
            key(KeyCode::Right, KeyModifiers::NONE, KeyEventKind::Press),
            Some(Message::ActionNext),
        ),
        (
            key(KeyCode::Enter, KeyModifiers::NONE, KeyEventKind::Press),
            Some(Message::Activate),
        ),
        (
            key(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL,
                KeyEventKind::Press,
            ),
            Some(Message::Quit),
        ),
    ] {
        assert_eq!(map_event(event), expected);
    }
}

#[test]
fn ignores_release_text_paste_and_unowned_pointer_input() {
    for event in [
        key(KeyCode::Enter, KeyModifiers::NONE, KeyEventKind::Release),
        key(KeyCode::Char('q'), KeyModifiers::NONE, KeyEventKind::Press),
        Event::Paste("not retained by the empty shell".to_owned()),
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 3,
            row: 4,
            modifiers: KeyModifiers::NONE,
        }),
    ] {
        assert_eq!(map_event(event), None);
    }
}

fn key(code: KeyCode, modifiers: KeyModifiers, kind: KeyEventKind) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers,
        kind,
        state: KeyEventState::NONE,
    })
}
