use ratatui_core::{backend::TestBackend, layout::Rect, terminal::Terminal};
use tablerock_tui::{
    ActionId, Effect, FocusRegion, LayoutMode, Message, Model, Screen, ShellView, update,
};
use termrock::runtime::{Dirty, drive_frame};

#[test]
fn reducer_owns_resize_focus_and_effects() {
    let mut model = Model::default();
    assert_eq!(
        update(
            &mut model,
            Message::Resize {
                width: 120,
                height: 40
            }
        )
        .dirty(),
        Dirty::Redraw
    );
    assert_eq!(model.layout_mode(), LayoutMode::Wide);
    assert_eq!(
        update(
            &mut model,
            Message::Resize {
                width: 120,
                height: 40
            }
        )
        .dirty(),
        Dirty::Clean
    );

    let _ = update(&mut model, Message::FocusNext);
    assert_eq!(model.focus(), FocusRegion::Catalog);
    let _ = update(&mut model, Message::FocusPrevious);
    assert_eq!(model.focus(), FocusRegion::Context);

    let result = update(&mut model, Message::Quit);
    assert_eq!(result.effects(), &[Effect::Exit]);
}

#[test]
fn actions_are_root_owned_and_only_activate_from_action_focus() {
    let mut model = Model::default();
    assert_eq!(
        update(&mut model, Message::ActionNext).dirty(),
        Dirty::Clean
    );
    assert_eq!(update(&mut model, Message::Activate).dirty(), Dirty::Clean);
    for _ in 0..4 {
        let _ = update(&mut model, Message::FocusNext);
    }
    assert_eq!(update(&mut model, Message::Activate).dirty(), Dirty::Redraw);
    assert_eq!(model.screen(), Screen::ConnectionPicker);
    let _ = update(&mut model, Message::ActionNext);
    assert_eq!(model.selected_action(), ActionId::Quit);
    assert_eq!(
        update(&mut model, Message::Activate).effects(),
        &[Effect::Exit]
    );
    let _ = update(&mut model, Message::ActionPrevious);
    assert_eq!(model.selected_action(), ActionId::Open);
}

#[test]
fn breakpoints_are_bounded_and_deterministic() {
    let mut model = Model::default();
    for (width, height, expected) in [
        (120, 40, LayoutMode::Wide),
        (80, 24, LayoutMode::Medium),
        (50, 18, LayoutMode::Narrow),
        (39, 18, LayoutMode::TooSmall),
        (80, 9, LayoutMode::TooSmall),
    ] {
        let _ = update(&mut model, Message::Resize { width, height });
        assert_eq!(model.layout_mode(), expected);
    }
}

#[test]
fn complete_view_renders_wide_and_minimum_states() {
    assert_render_contains(120, 24, &["Connections", "Catalog", "Workspace"]);
    assert_render_contains(80, 20, &["Connections", "Catalog", "Workspace"]);
    assert_render_contains(50, 18, &["Connections", "Open", "Ready"]);
    assert_render_contains(30, 8, &["Too Small"]);
}

#[test]
fn narrow_focus_projects_visible_regions_and_non_color_cues() {
    assert_render_after(
        50,
        18,
        &[Message::FocusNext],
        &["> Catalog", "Next focus", "Footer"],
    );
    assert_render_after(
        50,
        18,
        &[Message::FocusNext, Message::FocusNext],
        &["> Connections", "Next focus"],
    );
    assert_render_after(
        50,
        18,
        &[Message::FocusNext, Message::FocusNext, Message::FocusNext],
        &["> Workspace", "Next focus"],
    );
    assert_render_after(
        50,
        18,
        &[
            Message::FocusNext,
            Message::FocusNext,
            Message::FocusNext,
            Message::FocusNext,
        ],
        &["> Open", "Activate", "Choose action"],
    );
    assert_render_after(
        50,
        18,
        &[Message::FocusPrevious],
        &["[FOCUSED] Footer", "Next focus"],
    );
}

fn assert_render_contains(width: u16, height: u16, expected: &[&str]) {
    assert_render_after(width, height, &[], expected);
}

fn assert_render_after(width: u16, height: u16, messages: &[Message], expected: &[&str]) {
    let mut model = Model::default();
    let _ = update(&mut model, Message::Resize { width, height });
    for message in messages {
        let _ = update(&mut model, *message);
    }
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    drive_frame(
        &mut terminal,
        &ShellView,
        &model,
        Rect::new(0, 0, width, height),
        |_| {},
    )
    .expect("render frame");
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    for text in expected {
        assert!(
            rendered.contains(text),
            "missing {text:?} in rendered shell"
        );
    }
}
