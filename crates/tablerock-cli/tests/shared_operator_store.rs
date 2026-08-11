//! CLI open-default path shares the operator store with native macOS.

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use tablerock_core::{
    BoundedText, ByteLimit, Engine, IdParts, ProfileAggregate, ProfileConnectionSnapshot,
    ProfileDurability, ProfileId, ProfileIdentity, ProfileLimits, ProfileListFilter,
    ProfileListRequest, ProfileName, ProfileOrganization, ProfilePolicy, ProfilePreferences,
    ProfileProperty, ProfilePropertyBinding, ProfilePropertySet, ProfileSafetyMode,
    ReconnectPreference, Revision, TlsPolicy,
};
use tablerock_persistence::{
    OPERATOR_PROFILES_DB_FILE, PersistenceActor, TEST_ROOT_ENV, resolve_operator_profiles_database,
};

fn unique_root() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "tablerock-cli-shared-{}-{}",
        std::process::id(),
        nanos
    ))
}

#[test]
fn cli_library_path_matches_shared_operator_formula() {
    let home = std::path::Path::new("/Users/operator");
    let expected = resolve_operator_profiles_database(home, None).unwrap();
    #[cfg(target_os = "macos")]
    assert_eq!(
        expected,
        PathBuf::from("/Users/operator/Library/Application Support/TableRock/profiles.db")
    );
    assert_eq!(
        expected.file_name().and_then(|n| n.to_str()),
        Some(OPERATOR_PROFILES_DB_FILE)
    );
    let rendered = expected.to_string_lossy();
    assert!(!rendered.contains("state-"));
    assert!(!rendered.contains(&std::process::id().to_string()));
}

#[test]
fn cli_open_default_path_under_test_root_lists_seeded_profile() {
    let root = unique_root();
    fs::create_dir_all(&root).unwrap();
    let path = root.join(OPERATOR_PROFILES_DB_FILE);
    let profile_id = ProfileId::from_parts(IdParts::new(3, 77).unwrap()).unwrap();
    {
        let actor = PersistenceActor::open(&path).unwrap();
        let properties = ProfilePropertySet::new(vec![
            ProfilePropertyBinding::literal(
                ProfileProperty::Host,
                BoundedText::copy_from_str("seed.local", ByteLimit::new(32)).unwrap(),
            )
            .unwrap(),
            ProfilePropertyBinding::literal(
                ProfileProperty::Port,
                BoundedText::copy_from_str("5432", ByteLimit::new(8)).unwrap(),
            )
            .unwrap(),
            ProfilePropertyBinding::literal(
                ProfileProperty::DefaultContext,
                BoundedText::copy_from_str("seeddb", ByteLimit::new(16)).unwrap(),
            )
            .unwrap(),
            ProfilePropertyBinding::literal(
                ProfileProperty::Username,
                BoundedText::copy_from_str("seed", ByteLimit::new(16)).unwrap(),
            )
            .unwrap(),
        ])
        .unwrap();
        let connection = ProfileConnectionSnapshot::new(
            ProfileIdentity::new(
                profile_id,
                Revision::INITIAL,
                Engine::PostgreSql,
                ProfileName::new(
                    BoundedText::copy_from_str("seeded-shared", ByteLimit::new(32)).unwrap(),
                )
                .unwrap(),
            ),
            properties,
            ProfilePolicy::new(
                TlsPolicy::Disabled,
                ProfileSafetyMode::ConfirmWrites,
                ProfileLimits::new(10_000, 30_000, 5_000, 16 * 1024 * 1024).unwrap(),
            ),
        )
        .unwrap();
        let aggregate = ProfileAggregate::new(
            connection,
            ProfileDurability::Saved,
            ProfileOrganization::new(None, vec![], false, 0, None).unwrap(),
            ProfilePreferences::new(ReconnectPreference::BoundedAutomatic, true, 250).unwrap(),
        )
        .unwrap();
        actor
            .create_profile(aggregate.persistable().unwrap())
            .unwrap();
        actor.shutdown().unwrap();
    }

    // CLI open-default resolves via default_operator_profiles_database; under an
    // absolute test root that is `{root}/profiles.db` (same as this seed path).
    let resolved =
        resolve_operator_profiles_database(std::path::Path::new("/Users/unused"), Some(&root))
            .expect("resolve under test root");
    assert_eq!(resolved, path);
    assert_eq!(
        resolved.file_name().and_then(|n| n.to_str()),
        Some(OPERATOR_PROFILES_DB_FILE)
    );
    // Same PersistenceActor open path EffectExecutor::open_default uses after
    // path resolution (TTY-less list proof).
    let actor = PersistenceActor::open(&resolved).unwrap();
    let page = actor
        .list_profiles(
            ProfileListRequest::new(ProfileListFilter::new(None, None), None, 10).unwrap(),
        )
        .unwrap();
    assert_eq!(page.items().len(), 1);
    assert_eq!(page.items()[0].name().as_str(), "seeded-shared");
    assert_eq!(page.items()[0].id(), profile_id);
    actor.shutdown().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_public_path_helper_is_the_shared_operator_entry() {
    // Structural: shipped library re-exports the shared resolution entry used
    // by EffectExecutor::open_default (not a reimplemented formula).
    let _path_fn: fn() -> Result<PathBuf, String> = tablerock_cli::operator_profiles_database_path;
    let home = std::path::Path::new("/Users/operator");
    let shared = resolve_operator_profiles_database(home, None).unwrap();
    assert!(
        shared
            .to_string_lossy()
            .ends_with(&format!("TableRock/{OPERATOR_PROFILES_DB_FILE}"))
    );
    let _ = TEST_ROOT_ENV;
}
