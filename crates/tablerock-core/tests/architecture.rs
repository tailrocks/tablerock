#[test]
fn core_contract_has_no_runtime_or_presentation_dependency() {
    let manifest = include_str!("../Cargo.toml");
    // Core stays a pure contract crate. Its only workspace dependencies are the
    // pure parsing, normalization, and zeroizing crates below; bump the count
    // only when a deliberate core-contract decision adds another.
    for required in [
        "caseless.workspace = true",
        "sqlparser.workspace = true",
        "unicode-normalization.workspace = true",
        "zeroize.workspace = true",
    ] {
        assert!(
            manifest.contains(required),
            "core manifest must declare {required:?}"
        );
    }
    assert_eq!(manifest.matches(".workspace = true").count(), 9);

    let source = [
        include_str!("../src/lib.rs"),
        include_str!("../src/command.rs"),
        include_str!("../src/diagnostic.rs"),
        include_str!("../src/id.rs"),
        include_str!("../src/operation.rs"),
        include_str!("../src/page.rs"),
        include_str!("../src/profile.rs"),
        include_str!("../src/profile_aggregate.rs"),
        include_str!("../src/profile_list.rs"),
        include_str!("../src/revision.rs"),
        include_str!("../src/secret.rs"),
        include_str!("../src/sql_analysis.rs"),
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
            "core contract must not contain {forbidden:?}"
        );
    }
}
