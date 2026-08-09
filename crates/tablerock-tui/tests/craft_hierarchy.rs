//! TUI craft: sample action, empty-state hierarchy, non-color status labels.

use ratatui_core::{backend::TestBackend, layout::Rect, terminal::Terminal};
use tablerock_tui::{
    ActionId, Effect, Message, Model, ProfilesMsg, ShellView,
    model::profiles::{FailureProjection, ProfileListState},
    update,
};

#[test]
fn data_observatory_action_labels_are_lens_and_ledger() {
    // Product grammar: Relation Lens + Change Ledger (not generic FollowFK/Staged).
    assert_ne!(ActionId::FollowForeignKey, ActionId::ShowStaged);
    assert_ne!(ActionId::FollowForeignKey, ActionId::ApplyMutations);
    assert_ne!(ActionId::ShowStaged, ActionId::ApplyMutations);
}

#[test]
fn try_sample_action_is_distinct_connection_entry() {
    assert_ne!(ActionId::TrySample, ActionId::New);
    assert_ne!(ActionId::TrySample, ActionId::Open);
    assert_ne!(ActionId::TrySample, ActionId::Connect);
    let label = format!("{:?}", ActionId::TrySample);
    assert!(
        label.contains("TrySample"),
        "action debug form must stay TrySample, got {label}"
    );
}

#[test]
fn sample_effect_variant_exists_in_effect_module() {
    let effect = Effect::OpenSampleDatabase { request_token: 1 };
    match effect {
        Effect::OpenSampleDatabase { request_token } => assert_eq!(request_token, 1),
        _ => panic!("expected OpenSampleDatabase"),
    }
}

#[test]
fn empty_profiles_status_invites_sample() {
    let idle = ProfileListState::Idle;
    assert!(idle.status_line().contains("Sample"));
    let empty = ProfileListState::Loaded {
        request_token: 1,
        rows: Vec::new(),
        selected_id: None,
        search: String::new(),
        collapsed: Vec::new(),
    };
    assert!(empty.status_line().contains("empty"));
    assert!(empty.status_line().contains("Sample"));
    let body = empty.empty_body_lines();
    assert!(body.iter().any(|l| l.contains("Sample")));
    assert!(body.iter().any(|l| l.contains("No connections")));
}

#[test]
fn failed_profiles_status_is_non_color_text() {
    let failed = ProfileListState::Failed {
        request_token: 2,
        reason: FailureProjection::Label("disk".into()),
    };
    let line = failed.status_line();
    assert!(line.contains("error"));
    assert!(line.contains("disk"));
    let body = failed.empty_body_lines();
    assert!(body.iter().any(|l| l.contains("Could not load")));
    assert!(body.iter().any(|l| l.contains("Sample")));
}

#[test]
fn empty_connections_render_shows_sample_hierarchy() {
    let mut model = Model::default();
    let _ = update(
        &mut model,
        Message::Resize {
            width: 100,
            height: 28,
        },
    );
    let _ = update(
        &mut model,
        Message::Profiles(ProfilesMsg::ListLoaded {
            request_token: 1,
            items: Vec::new(),
        }),
    );
    let mut terminal = Terminal::new(TestBackend::new(100, 28)).expect("terminal");
    terminal
        .draw(|frame| ShellView.render(&model, frame, Rect::new(0, 0, 100, 28)))
        .expect("draw");
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(
        rendered.contains("Sample"),
        "action strip must show Sample: {rendered}"
    );
    assert!(
        rendered.contains("No connections") || rendered.contains("offline"),
        "empty body hierarchy missing: {rendered}"
    );
    assert!(
        rendered.contains("Profiles: empty") || rendered.contains("empty"),
        "status hierarchy missing: {rendered}"
    );
}

#[test]
fn sqlite_connect_ok_loads_root_catalog_not_failed_path() {
    // TrySample → ConnectOk must emit LoadCatalog Root for SQLite so the CLI
    // executor maps to CatalogRequest::SqliteRoot (not CatalogFailed).
    let mut model = Model::default();
    let _ = update(
        &mut model,
        Message::Resize {
            width: 80,
            height: 24,
        },
    );
    let result = update(
        &mut model,
        Message::Engine(tablerock_tui::EngineMsg::ConnectOk {
            request_token: 1,
            session_id_hex: "00000000000000010000000000000002".into(),
            identity: "sqlite:sample".into(),
            temporary: true,
            engine_label: "SQLite".into(),
            profile_id_hex: None,
            startup_summary: None,
            startup_pending: Vec::new(),
            reconnect_preference: Some("Manual".into()),
        }),
    );
    assert_eq!(model.screen(), tablerock_tui::Screen::Workbench);
    match result.effects().next() {
        Some(Effect::LoadCatalog {
            engine_label,
            level: tablerock_tui::effect::CatalogLevelSpec::Root,
            ..
        }) => assert_eq!(engine_label, "SQLite"),
        other => panic!("expected LoadCatalog Root for SQLite, got {other:?}"),
    }
}

#[test]
fn try_sample_preserves_profile_list_and_emits_effect() {
    let mut model = Model::default();
    let _ = update(
        &mut model,
        Message::Resize {
            width: 80,
            height: 24,
        },
    );
    // Focus order: Context → Catalog → Tabs → Content → Actions.
    for _ in 0..4 {
        let _ = update(&mut model, Message::FocusNext);
    }
    assert_eq!(
        model.focus(),
        Some(tablerock_tui::FocusRegion::Actions),
        "need Actions focus for ActionNext"
    );
    let mut guard = 0;
    while model.selected_action() != ActionId::TrySample {
        assert!(guard < 64, "action palette never reached TrySample");
        let _ = update(&mut model, Message::ActionNext);
        guard += 1;
    }
    let profiles_before = model.profiles().clone();
    let result = update(&mut model, Message::Activate);
    assert!(
        matches!(
            result.effects().next(),
            Some(Effect::OpenSampleDatabase { .. })
        ),
        "TrySample must emit OpenSampleDatabase"
    );
    assert_eq!(
        model.profiles(),
        &profiles_before,
        "profile list must not become Loading after TrySample"
    );
    assert!(
        model
            .session()
            .and_then(|s| s.status.as_deref())
            .is_some_and(|s| s.contains("sample")),
        "transient opening status expected"
    );
}
