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
