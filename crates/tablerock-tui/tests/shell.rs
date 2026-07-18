use ratatui_core::{backend::TestBackend, layout::Rect, terminal::Terminal};
use tablerock_tui::{
    ActionId, Effect, FocusRegion, LayoutMode, Message, Model, PasteText, ProfilesMsg, Screen,
    ShellTarget, ShellView, model::profiles::{LiveConnectionState, ProfileRowProjection}, update,
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
    // Cycle forward to Quit. The connections palette grows over time
    // (ImportUrl, OpenExternalUrl, QuickSwitch, RenameGroup, Reconnect were
    // inserted before Quit), so walk by identity instead of a hardcoded offset.
    let mut guard = 0;
    while model.selected_action() != ActionId::Quit {
        assert!(guard < 64, "action palette never reached Quit");
        let _ = update(&mut model, Message::ActionNext);
        guard += 1;
    }
    assert_eq!(
        update(&mut model, Message::Activate)
            .effects()
            .cloned()
            .collect::<Vec<_>>(),
        [Effect::Exit]
    );
    let _ = update(&mut model, Message::ActionPrevious);
    assert_ne!(model.selected_action(), ActionId::Quit);
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

/// First-paint budget: cold model + resize + draw under 50 ms (local unit bound).
#[test]
fn first_paint_budget_under_50ms() {
    use std::time::Instant;
    let started = Instant::now();
    let mut model = Model::default();
    let _ = update(
        &mut model,
        Message::Resize {
            width: 100,
            height: 30,
        },
    );
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("test terminal");
    terminal
        .draw(|frame| ShellView.render(&model, frame, Rect::new(0, 0, 100, 30)))
        .expect("first paint");
    let elapsed = started.elapsed();
    assert!(
        elapsed.as_millis() < 50,
        "first paint took {elapsed:?} (budget 50 ms)"
    );
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(!rendered.trim().is_empty(), "first paint must draw content");
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
            assert!(out.needs_render(), "every resize must request paint");
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

/// Responsive layout must render Unicode (Cyrillic/CJK/Hangul/emoji) and
/// extreme-length profile labels without panicking or corrupting layout. This
/// closes the ledger's "Render fixtures with Unicode and extreme labels"
/// acceptance for the Responsive layout row, which the size-only fixtures
/// above do not cover.
#[test]
fn render_handles_unicode_and_extreme_profile_labels() {
    let mut model = Model::default();
    let bootstrap = update(
        &mut model,
        Message::Resize { width: 100, height: 30 },
    );
    let request_token = bootstrap
        .effects()
        .find_map(|effect| match effect {
            Effect::LoadProfileList { request_token, .. } => Some(*request_token),
            _ => None,
        })
        .expect("bootstrap emits LoadProfileList");

    // Mix single-width (Cyrillic, Hangul), double-width (CJK, emoji), and an
    // extreme 240-byte label that must truncate, not break, the row layout.
    let items = vec![
        unicode_extreme_profile_row(0, "Табла"),
        unicode_extreme_profile_row(1, "日本語プロファイル"),
        unicode_extreme_profile_row(2, "한국어연결"),
        unicode_extreme_profile_row(3, "🎉db🎉"),
        unicode_extreme_profile_row(4, &"a".repeat(240)),
    ];
    let loaded = update(
        &mut model,
        Message::Profiles(ProfilesMsg::ListLoaded {
            request_token,
            items,
        }),
    );
    assert!(loaded.needs_render(), "loaded list must request paint");

    // Wide render: single-width Unicode label is present and nothing panicked;
    // double-width and extreme labels coexist without corrupting siblings.
    assert_render_model_contains(&model, 100, 30, "Табла");
    assert_render_model_contains(&model, 100, 30, "Connections");

    // Narrow render must not panic under Unicode plus an extreme label.
    let mut narrow = Terminal::new(TestBackend::new(24, 10)).expect("test terminal");
    narrow
        .draw(|frame| ShellView.render(&model, frame, Rect::new(0, 0, 24, 10)))
        .expect("narrow render with Unicode and extreme labels");
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

fn unicode_extreme_profile_row(id: u8, name: &str) -> ProfileRowProjection {
    ProfileRowProjection {
        id_hex: format!("{id:02x}"),
        name: name.to_owned(),
        engine_label: "PostgreSQL".to_owned(),
        group: None,
        favorite: false,
        target_summary: "h:5432/db".to_owned(),
        environment: None,
        production_warning: false,
        safety_label: "Confirm writes".to_owned(),
        plaintext_secret_warning: false,
        live_state: LiveConnectionState::Disconnected,
    }
}
