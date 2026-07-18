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
fn tui_manifest_exposes_only_presentation_dependencies() {
    // rust-core-architecture.md: tablerock-tui → tablerock-core + termrock.
    // No engine/persistence/async runtime edge from presentation.
    let manifest = include_str!("../Cargo.toml");
    let dependencies = manifest
        .split_once("[dependencies]")
        .expect("dependency section")
        .1
        .split_once("[lints]")
        .expect("lints section")
        .0;
    let deps = dependencies.trim();
    assert!(
        deps.contains("ratatui-core.workspace = true"),
        "tui must depend on ratatui-core"
    );
    assert!(
        deps.contains("termrock.workspace = true"),
        "tui must depend on termrock"
    );
    assert!(
        deps.contains("tablerock-core"),
        "tui may depend on pure tablerock-core contracts"
    );
    for forbidden in [
        "tablerock-engine",
        "tablerock-persistence",
        "tablerock-cli",
        "tokio",
        "sqlx",
        "rusqlite",
    ] {
        assert!(
            !deps.contains(forbidden),
            "tui must not depend on {forbidden}"
        );
    }
}
