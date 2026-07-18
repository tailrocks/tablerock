#[test]
fn reducer_and_view_sources_exclude_io_and_async_capabilities() {
    let sources = [
        ("update.rs", include_str!("../src/update.rs")),
        ("view.rs", include_str!("../src/view.rs")),
    ];
    let forbidden = [
        ".await",
        "tokio::",
        "std::fs",
        "std::io",
        "std::net",
        "std::process",
        "std::thread",
        "std::time",
        "SystemTime",
        "Instant",
        "tracing::",
        "telemetry",
    ];
    for (name, source) in sources {
        for token in forbidden {
            assert!(
                !source.contains(token),
                "{name} must not contain I/O or async capability {token:?}"
            );
        }
    }
}

#[test]
fn connection_screens_use_termrock_form_and_tree() {
    let view = include_str!("../src/view.rs");
    assert!(
        view.contains("Form::new") && view.contains("Tree::new"),
        "connection screens must render TermRock Form and Tree"
    );
    // No local generic form/tree widgets (TableRock-local form model is ok).
    for (name, source) in [
        ("view.rs", include_str!("../src/view.rs")),
        ("editor.rs", include_str!("../src/model/editor.rs")),
        ("profiles.rs", include_str!("../src/model/profiles.rs")),
    ] {
        assert!(
            !source.contains("struct Form ") && !source.contains("struct Tree "),
            "{name} must not define a local Form/Tree widget"
        );
    }
}

#[test]
fn tui_manifest_exposes_only_rendering_dependencies() {
    let manifest = include_str!("../Cargo.toml");
    let dependencies = manifest
        .split_once("[dependencies]")
        .expect("dependency section")
        .1
        .split_once("[lints]")
        .expect("lints section")
        .0;
    assert_eq!(
        dependencies.trim(),
        "ratatui-core.workspace = true\ntermrock.workspace = true"
    );
}
