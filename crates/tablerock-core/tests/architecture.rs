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

    // Inspect every source file. A hand-maintained include list let new modules
    // bypass this guard, while engine-name substrings incorrectly rejected
    // valid engine-typed contracts. Purity is about dependencies and I/O APIs,
    // not domain vocabulary.
    let source_directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let source = std::fs::read_dir(source_directory)
        .expect("core source directory")
        .map(|entry| entry.expect("core source entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .map(|path| std::fs::read_to_string(path).expect("read core source"))
        .collect::<String>();
    for forbidden in [
        "tokio",
        "ratatui",
        "termrock",
        "crossterm",
        "std::time",
        "std::net",
    ] {
        assert!(
            !source.contains(forbidden),
            "core contract must not contain {forbidden:?}"
        );
    }
}
