#[test]
fn core_contract_has_no_runtime_or_presentation_dependency() {
    let manifest = include_str!("../Cargo.toml");
    assert!(!manifest.contains("[dependencies]"));

    let source = [
        include_str!("../src/lib.rs"),
        include_str!("../src/id.rs"),
        include_str!("../src/revision.rs"),
        include_str!("../src/value.rs"),
    ]
    .concat();
    for forbidden in [
        "tokio",
        "ratatui",
        "termrock",
        "crossterm",
        "postgres",
        "clickhouse",
        "redis",
        "std::time",
        "std::net",
    ] {
        assert!(
            !source.contains(forbidden),
            "core identity contract must not contain {forbidden:?}"
        );
    }
}
