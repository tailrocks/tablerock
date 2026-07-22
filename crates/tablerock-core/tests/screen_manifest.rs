use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const MANIFEST: &str = include_str!("../../../docs/architecture/screen-manifest.tsv");
const STATE_PROFILES: &str = include_str!("../../../docs/architecture/screen-state-profiles.tsv");
const CLIENT_STATUS: &str = include_str!("../../../docs/architecture/screen-client-status.tsv");

const PRODUCT_DOCS: &[&str] = &[
    "docs/product/clickhouse.md",
    "docs/product/connections.md",
    "docs/product/copy-export.md",
    "docs/product/data-grid.md",
    "docs/product/editing.md",
    "docs/product/native-macos.md",
    "docs/product/redis.md",
    "docs/product/sql-editor.md",
    "docs/product/workbench.md",
];

const REQUIRED_STATES: &[&str] = &[
    "normal",
    "empty",
    "loading",
    "partial",
    "stale",
    "disabled",
    "unsupported",
    "validation",
    "permission",
    "disconnected",
    "reconnecting",
    "error",
    "destructive-confirmation",
    "narrow",
    "large-data",
    "recovery",
];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn assert_path(root: &Path, value: &str, context: &str) {
    if value == "n/a" {
        return;
    }
    assert!(
        root.join(value).is_file(),
        "{context} references stale path {value}"
    );
}

fn links(value: &str) -> BTreeMap<&str, &str> {
    value
        .split(';')
        .map(|link| {
            link.split_once(':')
                .unwrap_or_else(|| panic!("link lacks owner prefix: {link}"))
        })
        .collect()
}

#[test]
fn canonical_screen_manifest_is_structurally_closed() {
    let root = repository_root();
    let client_status: BTreeMap<_, _> = CLIENT_STATUS
        .lines()
        .skip(1)
        .map(|line| {
            let columns: Vec<_> = line.split('|').collect();
            assert_eq!(columns.len(), 3, "client-status row must have three columns");
            let statuses = (columns[1], columns[2]);
            for status in [statuses.0, statuses.1] {
                assert!(
                    ["proven", "partial", "missing", "n/a"].contains(&status),
                    "{} has bad client status {status}",
                    columns[0]
                );
            }
            (columns[0], statuses)
        })
        .collect();
    let profiles: BTreeMap<_, _> = STATE_PROFILES
        .lines()
        .skip(1)
        .map(|line| line.split_once('|').expect("state profile row"))
        .collect();
    assert!(!profiles.is_empty());
    for (name, states) in &profiles {
        let states: BTreeSet<_> = states.split(',').collect();
        for required in REQUIRED_STATES {
            assert!(states.contains(required), "profile {name} lacks {required}");
        }
    }

    let header = MANIFEST.lines().next().expect("manifest header");
    assert_eq!(header.split('|').count(), 16);
    let mut ids = BTreeSet::new();
    let mut covered_product_docs = BTreeSet::new();
    let mut rows = 0usize;

    for (offset, line) in MANIFEST.lines().skip(1).enumerate() {
        let columns: Vec<_> = line.split('|').collect();
        assert_eq!(columns.len(), 16, "manifest line {}", offset + 2);
        let [
            id,
            surface,
            kind,
            clients,
            engines,
            entry_exit,
            actions,
            focus,
            state_profile,
            requirement,
            plan,
            implementation,
            tests,
            evidence,
            status,
            gap,
        ] = columns.as_slice()
        else {
            unreachable!()
        };
        assert!(id.starts_with("TR-SCR-"), "unstable screen id {id}");
        assert!(ids.insert(*id), "duplicate screen id {id}");
        let (tui_status, native_status) = client_status
            .get(id)
            .copied()
            .unwrap_or_else(|| panic!("{id} lacks client-specific status"));
        for (name, value) in [
            ("surface", surface),
            ("kind", kind),
            ("engines", engines),
            ("entry_exit", entry_exit),
            ("actions", actions),
            ("focus", focus),
            ("gap", gap),
        ] {
            assert!(!value.is_empty(), "{id} lacks {name}");
        }
        assert!(
            profiles.contains_key(state_profile),
            "{id} has unknown state profile"
        );
        assert!(
            ["both", "tui", "native"].contains(clients),
            "{id} has bad clients"
        );
        assert!(
            ["partial", "proven", "missing"].contains(status),
            "{id} has bad status"
        );
        assert_path(&root, requirement, id);
        assert_path(&root, plan, id);
        assert_path(&root, evidence, id);
        covered_product_docs.insert(*requirement);

        let implementation = links(implementation);
        let tests = links(tests);
        for owner in ["core", "tui", "native"] {
            let path = implementation
                .get(owner)
                .unwrap_or_else(|| panic!("{id} lacks {owner} implementation link"));
            assert_path(&root, path, id);
        }
        for owner in ["tui", "native"] {
            let path = tests
                .get(owner)
                .unwrap_or_else(|| panic!("{id} lacks {owner} test link"));
            assert_path(&root, path, id);
        }
        match *clients {
            "both" => {
                assert_ne!(tui_status, "n/a", "{id} marks applicable TUI n/a");
                assert_ne!(native_status, "n/a", "{id} marks applicable native n/a");
            }
            "tui" => {
                assert_ne!(tui_status, "n/a", "{id} marks applicable TUI n/a");
                assert_eq!(native_status, "n/a", "{id} marks inapplicable native active");
            }
            "native" => {
                assert_eq!(tui_status, "n/a", "{id} marks inapplicable TUI active");
                assert_ne!(native_status, "n/a", "{id} marks applicable native n/a");
            }
            _ => unreachable!(),
        }
        for (owner, owner_status) in [("tui", tui_status), ("native", native_status)] {
            if owner_status == "missing" || owner_status == "n/a" {
                assert_eq!(
                    implementation[owner], "n/a",
                    "{id} {owner} status is {owner_status} but implementation is linked"
                );
                assert_eq!(
                    tests[owner], "n/a",
                    "{id} {owner} status is {owner_status} but test is linked"
                );
            } else {
                assert_ne!(
                    implementation[owner], "n/a",
                    "{id} {owner} status is {owner_status} without implementation"
                );
                assert_ne!(
                    tests[owner], "n/a",
                    "{id} {owner} status is {owner_status} without test"
                );
            }
        }
        let applicable_statuses: Vec<_> = [tui_status, native_status]
            .into_iter()
            .filter(|value| *value != "n/a")
            .collect();
        let expected_status = if applicable_statuses.contains(&"missing") {
            "missing"
        } else if applicable_statuses.iter().all(|value| *value == "proven") {
            "proven"
        } else {
            "partial"
        };
        assert_eq!(
            *status, expected_status,
            "{id} aggregate status disagrees with client-specific status"
        );
        if *clients == "both" && *status != "missing" {
            assert_ne!(
                implementation["tui"], "n/a",
                "{id} lacks TUI implementation"
            );
            assert_ne!(
                implementation["native"], "n/a",
                "{id} lacks native implementation"
            );
            assert_ne!(tests["tui"], "n/a", "{id} lacks TUI test");
            assert_ne!(tests["native"], "n/a", "{id} lacks native test");
        }
        if *status == "missing" {
            assert!(
                implementation.values().any(|path| *path == "n/a")
                    || tests.values().any(|path| *path == "n/a"),
                "{id} claims missing without an explicit missing implementation or test"
            );
        }
        if *status == "proven" {
            assert_eq!(*gap, "none", "{id} claims proven with an open gap");
            assert_ne!(*evidence, "n/a", "{id} claims proven without evidence");
        } else {
            assert_ne!(*gap, "none", "{id} has no honest open gap");
        }
        rows += 1;
    }

    assert!(
        rows >= 38,
        "canonical surface inventory unexpectedly shrank"
    );
    assert_eq!(
        client_status.len(),
        ids.len(),
        "client-status inventory disagrees with screen manifest"
    );
    for id in client_status.keys() {
        assert!(ids.contains(id), "client-status row has unknown screen id {id}");
    }
    for product_doc in PRODUCT_DOCS {
        assert!(
            covered_product_docs.contains(product_doc),
            "product document lacks a manifest row: {product_doc}"
        );
    }
}
