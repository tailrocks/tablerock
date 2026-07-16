use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use ratatui_core::{backend::TestBackend, layout::Rect, terminal::Terminal};
use tablerock_cli::{InputAdapter, map_backend_event, map_event};
use tablerock_tui::{
    ActionId, Message, Model, ScrollDirection, ShellKeyAction, ShellTarget, ShellView, update,
};
use termrock::input::{
    Event as TermRockEvent, KeyCode as TermRockKeyCode, KeyEvent as TermRockKeyEvent,
    KeyModifiers as TermRockKeyModifiers,
};
use termrock::keymap::KeyChord;

#[test]
fn maps_termrock_neutral_input_without_backend_types() {
    assert_eq!(
        map_event(TermRockEvent::Resize {
            width: 100,
            height: 40,
        }),
        Some(Message::Resize {
            width: 100,
            height: 40,
        })
    );
    assert_eq!(
        map_event(TermRockEvent::Key(TermRockKeyEvent::new(
            TermRockKeyCode::Enter,
            TermRockKeyModifiers::NONE,
        ))),
        Some(Message::Activate)
    );
    let Message::Paste(paste) =
        map_event(TermRockEvent::Paste("neutral paste".to_owned())).unwrap()
    else {
        panic!("expected neutral paste intent");
    };
    assert_eq!(paste.text(), "neutral paste");
}

#[test]
fn maps_backend_facts_and_semantic_keyboard_intents() {
    for (event, expected) in [
        (
            Event::Resize(80, 24),
            Some(Message::Resize {
                width: 80,
                height: 24,
            }),
        ),
        (
            Event::FocusGained,
            Some(Message::TerminalFocusChanged(true)),
        ),
        (Event::FocusLost, Some(Message::TerminalFocusChanged(false))),
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
        assert_eq!(map_backend_event(event), expected);
    }
}

#[test]
fn bounds_and_redacts_paste_before_root_delivery() {
    let secret = "password=correct-horse";
    let message = map_backend_event(Event::Paste(secret.to_owned())).expect("paste fact");
    let Message::Paste(paste) = message else {
        panic!("expected paste fact");
    };
    assert_eq!(paste.text(), secret);
    assert!(!paste.was_truncated());
    assert!(!format!("{paste:?}").contains(secret));

    let oversized = "🪨".repeat(tablerock_tui::MAX_PASTE_BYTES / 2);
    let Message::Paste(paste) = map_backend_event(Event::Paste(oversized)).expect("bounded paste")
    else {
        panic!("expected paste fact");
    };
    assert!(paste.was_truncated());
    assert!(paste.text().len() <= tablerock_tui::MAX_PASTE_BYTES);
    assert!(paste.text().is_char_boundary(paste.text().len()));
}

#[test]
fn maps_pointer_input_only_through_painted_geometry() {
    let mut adapter = rendered_adapter(80, 24);
    let open = (1, 21);
    assert_eq!(
        adapter.map_backend_event(mouse(MouseEventKind::Down(MouseButton::Left), open)),
        Some(Message::PointerPressed(Some(ShellTarget::Action(
            ActionId::Open
        ))))
    );
    assert_eq!(
        adapter.map_backend_event(mouse(MouseEventKind::Up(MouseButton::Left), open)),
        Some(Message::PointerReleased(Some(ShellTarget::Action(
            ActionId::Open
        ))))
    );
    assert_eq!(
        adapter.map_backend_event(mouse(MouseEventKind::ScrollDown, (40, 10))),
        Some(Message::PointerScrolled {
            target: Some(ShellTarget::Focus(tablerock_tui::FocusRegion::Content)),
            direction: ScrollDirection::Down,
        })
    );

    adapter.set_geometry(Default::default());
    assert_eq!(
        adapter.map_backend_event(mouse(MouseEventKind::Moved, open)),
        Some(Message::PointerHovered(None))
    );
}

#[test]
fn ignores_key_release_unbound_keys_and_non_primary_buttons() {
    for event in [
        key(KeyCode::Enter, KeyModifiers::NONE, KeyEventKind::Release),
        key(KeyCode::Char('q'), KeyModifiers::NONE, KeyEventKind::Press),
        mouse(MouseEventKind::Down(MouseButton::Right), (3, 4)),
    ] {
        assert_eq!(map_backend_event(event), None);
    }
}

#[test]
fn runtime_remap_drives_dispatch_and_advertised_hint_from_one_keymap() {
    let mut model = Model::default();
    assert!(model.keymap_mut().remap(
        ShellKeyAction::Activate,
        vec![KeyChord::ctrl(TermRockKeyCode::Char('l'))],
    ));
    let adapter = InputAdapter::default();
    assert_eq!(
        adapter.map_event_with_keymap(
            TermRockEvent::Key(TermRockKeyEvent::new(
                TermRockKeyCode::Enter,
                TermRockKeyModifiers::NONE,
            )),
            model.keymap(),
        ),
        None
    );
    assert_eq!(
        adapter.map_event_with_keymap(
            TermRockEvent::Key(TermRockKeyEvent::new(
                TermRockKeyCode::Char('l'),
                TermRockKeyModifiers::CONTROL,
            )),
            model.keymap(),
        ),
        Some(Message::Activate)
    );
    assert_eq!(model.keymap().glyph_for(ShellKeyAction::Activate), "Ctrl-L");

    let _ = update(
        &mut model,
        Message::Resize {
            width: 120,
            height: 24,
        },
    );
    for _ in 0..4 {
        let _ = update(&mut model, Message::FocusNext);
    }
    let mut terminal = Terminal::new(TestBackend::new(120, 24)).unwrap();
    terminal
        .draw(|frame| ShellView.render(&model, frame, frame.area()))
        .unwrap();
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("Ctrl-L"));
}

fn rendered_adapter(width: u16, height: u16) -> InputAdapter {
    let mut model = Model::default();
    let _ = update(&mut model, Message::Resize { width, height });
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    let mut geometry = None;
    terminal
        .draw(|frame| {
            geometry =
                Some(ShellView.render_with_geometry(&model, frame, Rect::new(0, 0, width, height)));
        })
        .expect("render shell geometry");
    let mut adapter = InputAdapter::default();
    adapter.set_geometry(geometry.expect("painted geometry"));
    adapter
}

fn mouse(kind: MouseEventKind, (column, row): (u16, u16)) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn key(code: KeyCode, modifiers: KeyModifiers, kind: KeyEventKind) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers,
        kind,
        state: KeyEventState::NONE,
    })
}
