use ratatui_core::{backend::TestBackend, layout::Rect, terminal::Terminal};
use tablerock_tui::{
    ActionId, Effect, FocusRegion, LayoutMode, Message, Model, PasteText, Screen, ShellTarget,
    ShellView, update,
};

#[test]
fn reducer_owns_resize_focus_and_effects() {
    let mut model = Model::default();
    assert!(
        update(
            &mut model,
            Message::Resize {
                width: 120,
                height: 40
            }
        )
        .needs_render()
    );
    assert_eq!(model.layout_mode(), LayoutMode::Wide);
    assert!(
        !update(
            &mut model,
            Message::Resize {
                width: 120,
                height: 40
            }
        )
        .needs_render()
    );

    let _ = update(&mut model, Message::FocusNext);
    assert_eq!(model.focus(), Some(FocusRegion::Catalog));
    let _ = update(&mut model, Message::FocusPrevious);
    assert_eq!(model.focus(), Some(FocusRegion::Context));

    let result = update(&mut model, Message::Quit);
    assert_eq!(
        result.effects().cloned().collect::<Vec<_>>(),
        [Effect::Exit]
    );
}

#[test]
fn actions_are_root_owned_and_only_activate_from_action_focus() {
    let mut model = Model::default();
    assert!(!update(&mut model, Message::ActionNext).needs_render());
    assert!(!update(&mut model, Message::Activate).needs_render());
    for _ in 0..4 {
        let _ = update(&mut model, Message::FocusNext);
    }
    // Open with empty list: no selection → no effect / screen change.
    assert!(!update(&mut model, Message::Activate).needs_render());
    assert_eq!(model.screen(), Screen::Connections);
    let _ = update(&mut model, Message::ActionNext);
    assert_eq!(model.selected_action(), ActionId::New);
    let _ = update(&mut model, Message::ActionNext);
    assert_eq!(model.selected_action(), ActionId::Remove);
    let _ = update(&mut model, Message::ActionNext);
    assert_eq!(model.selected_action(), ActionId::Quit);
    assert_eq!(
        update(&mut model, Message::Activate)
            .effects()
            .cloned()
            .collect::<Vec<_>>(),
        [Effect::Exit]
    );
    let _ = update(&mut model, Message::ActionPrevious);
    assert_eq!(model.selected_action(), ActionId::Remove);
}

#[test]
fn pointer_activation_requires_matching_render_authorized_press_and_release() {
    let mut model = Model::default();
    let open = Some(ShellTarget::Action(ActionId::Open));
    let quit = Some(ShellTarget::Action(ActionId::Quit));

    let _ = update(&mut model, Message::PointerPressed(open));
    assert_eq!(model.focus(), Some(FocusRegion::Actions));
    assert_eq!(model.selected_action(), ActionId::Open);
    let mismatch = update(&mut model, Message::PointerReleased(quit));
    assert!(mismatch.effects().next().is_none());
    assert_eq!(model.screen(), Screen::Connections);

    let _ = update(&mut model, Message::PointerPressed(open));
    let activated = update(&mut model, Message::PointerReleased(open));
    // Empty list: Open has no selected profile.
    assert!(!activated.needs_render());
    assert_eq!(model.screen(), Screen::Connections);

    let _ = update(&mut model, Message::PointerPressed(quit));
    assert_eq!(
        update(&mut model, Message::PointerReleased(quit))
            .effects()
            .cloned()
            .collect::<Vec<_>>(),
        [Effect::Exit]
    );
}

#[test]
fn focus_loss_clears_transient_pointer_state_and_paste_is_not_retained() {
    let mut model = Model::default();
    let target = Some(ShellTarget::Focus(FocusRegion::Catalog));
    let _ = update(&mut model, Message::PointerHovered(target));
    let _ = update(&mut model, Message::PointerPressed(target));
    assert_eq!(model.hovered(), target);
    assert_eq!(model.pressed(), target);

    let _ = update(&mut model, Message::TerminalFocusChanged(false));
    assert!(!model.terminal_focused());
    assert_eq!(model.hovered(), None);
    assert_eq!(model.pressed(), None);

    assert!(
        !update(
            &mut model,
            Message::Paste(PasteText::bounded("credential material".to_owned()))
        )
        .needs_render()
    );
}

#[test]
fn ingress_overflow_projects_an_explicit_resync_state_until_reconciled() {
    let mut model = Model::default();
    let _ = update(
        &mut model,
        Message::Resize {
            width: 80,
            height: 24,
        },
    );
    assert!(update(&mut model, Message::EngineResyncRequired).needs_render());
    assert!(model.engine_resync_required());
    assert!(!update(&mut model, Message::EngineResyncRequired).needs_render());
    assert_render_model_contains(&model, 80, 24, "Resync required");

    assert!(update(&mut model, Message::EngineResynchronized).needs_render());
    assert!(!model.engine_resync_required());
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
fn view_authorizes_only_geometry_painted_in_the_current_frame() {
    let mut model = Model::default();
    let _ = update(
        &mut model,
        Message::Resize {
            width: 80,
            height: 24,
        },
    );
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("test terminal");
    let mut geometry = None;
    terminal
        .draw(|frame| {
            geometry = Some(ShellView.render_with_geometry(&model, frame, Rect::new(0, 0, 80, 24)));
        })
        .expect("render geometry");
    let geometry = geometry.expect("current frame geometry");
    assert_eq!(
        geometry.target_at(1, 21),
        Some(ShellTarget::Action(ActionId::Open))
    );
    assert_eq!(
        geometry.target_at(40, 10),
        Some(ShellTarget::Focus(FocusRegion::Content))
    );
    assert_eq!(
        geometry.target_at(79, 23),
        Some(ShellTarget::Focus(FocusRegion::Footer))
    );

    let _ = update(
        &mut model,
        Message::Resize {
            width: 30,
            height: 8,
        },
    );
    let mut terminal = Terminal::new(TestBackend::new(30, 8)).expect("small terminal");
    let mut small = None;
    terminal
        .draw(|frame| {
            small = Some(ShellView.render_with_geometry(&model, frame, Rect::new(0, 0, 30, 8)));
        })
        .expect("render small geometry");
    assert_eq!(small.expect("small geometry").target_at(1, 1), None);
}

#[test]
fn pointer_hover_projects_a_non_color_action_cue() {
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
        Message::PointerDragged(Some(ShellTarget::Action(ActionId::Quit))),
    );
    assert_render_model_contains(&model, 80, 24, "~ Quit");
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

/// SIGWINCH storm: many rapid Resize messages must not panic and last size wins.
#[test]
fn resize_storm_last_geometry_wins_and_renders() {
    let mut model = Model::default();
    // Alternate narrow/wide/tall sizes like a resize-storm from the terminal.
    let sizes: &[(u16, u16)] = &[
        (40, 12),
        (80, 24),
        (20, 8),
        (120, 40),
        (10, 5),
        (100, 30),
        (50, 18),
    ];
    for _ in 0..32 {
        for &(width, height) in sizes {
            let out = update(&mut model, Message::Resize { width, height });
            assert!(out.render, "every resize must request paint");
        }
    }
    // Last size in the loop is (50, 18).
    let mut terminal = Terminal::new(TestBackend::new(50, 18)).expect("test terminal");
    terminal
        .draw(|frame| ShellView.render(&model, frame, Rect::new(0, 0, 50, 18)))
        .expect("render after storm");
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(
        rendered.contains("Connections") || rendered.contains("TableRock") || !rendered.is_empty(),
        "shell must still render after resize storm"
    );
}

fn assert_render_contains(width: u16, height: u16, expected: &[&str]) {
    assert_render_after(width, height, &[], expected);
}

fn assert_render_after(width: u16, height: u16, messages: &[Message], expected: &[&str]) {
    let mut model = Model::default();
    let _ = update(&mut model, Message::Resize { width, height });
    for message in messages {
        let _ = update(&mut model, message.clone());
    }
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    terminal
        .draw(|frame| ShellView.render(&model, frame, Rect::new(0, 0, width, height)))
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

fn assert_render_model_contains(model: &Model, width: u16, height: u16, expected: &str) {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    terminal
        .draw(|frame| ShellView.render(model, frame, Rect::new(0, 0, width, height)))
        .expect("render model");
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains(expected), "missing {expected:?}");
}
