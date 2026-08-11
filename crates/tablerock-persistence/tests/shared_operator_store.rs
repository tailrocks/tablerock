//! Shared operator store: CLI open-default path + sequential multi-client use.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use tablerock_core::{
    BoundedText, ByteLimit, Engine, EnvironmentTag, IdParts, ProfileAggregate,
    ProfileConnectionSnapshot, ProfileDurability, ProfileGroupName, ProfileId, ProfileIdentity,
    ProfileLimits, ProfileListFilter, ProfileListPage, ProfileListRequest, ProfileName,
    ProfileOrganization, ProfilePolicy, ProfilePreferences, ProfileProperty,
    ProfilePropertyBinding, ProfilePropertySet, ProfileSafetyMode, ProfileTag, ReconnectPreference,
    TlsPolicy,
};
use tablerock_persistence::{
    OPERATOR_PROFILES_DB_FILE, PersistenceActor, default_operator_profiles_database,
    resolve_operator_profiles_database,
};

fn unique_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "tablerock-shared-store-{}-{}-{}",
        label,
        std::process::id(),
        nanos
    ))
}

fn sample_profile(id: ProfileId, name: &str) -> ProfileAggregate {
    let properties = ProfilePropertySet::new(vec![
        ProfilePropertyBinding::literal(
            ProfileProperty::Host,
            BoundedText::copy_from_str("127.0.0.1", ByteLimit::new(16)).unwrap(),
        )
        .unwrap(),
        ProfilePropertyBinding::literal(
            ProfileProperty::Port,
            BoundedText::copy_from_str("5432", ByteLimit::new(8)).unwrap(),
        )
        .unwrap(),
        ProfilePropertyBinding::literal(
            ProfileProperty::DefaultContext,
            BoundedText::copy_from_str("postgres", ByteLimit::new(16)).unwrap(),
        )
        .unwrap(),
        ProfilePropertyBinding::literal(
            ProfileProperty::Username,
            BoundedText::copy_from_str("operator", ByteLimit::new(16)).unwrap(),
        )
        .unwrap(),
    ])
    .unwrap();
    let connection = ProfileConnectionSnapshot::new(
        ProfileIdentity::new(
            id,
            tablerock_core::Revision::INITIAL,
            Engine::PostgreSql,
            ProfileName::new(BoundedText::copy_from_str(name, ByteLimit::new(64)).unwrap())
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
    ProfileAggregate::new(
        connection,
        ProfileDurability::Saved,
        ProfileOrganization::new(
            Some(
                ProfileGroupName::new(
                    BoundedText::copy_from_str("shared", ByteLimit::new(16)).unwrap(),
                )
                .unwrap(),
            ),
            vec![
                ProfileTag::new(BoundedText::copy_from_str("parity", ByteLimit::new(16)).unwrap())
                    .unwrap(),
            ],
            true,
            0,
            Some(EnvironmentTag::Development),
        )
        .unwrap(),
        ProfilePreferences::new(ReconnectPreference::BoundedAutomatic, true, 250).unwrap(),
    )
    .unwrap()
}

fn list_all(actor: &PersistenceActor) -> ProfileListPage {
    let request = ProfileListRequest::new(ProfileListFilter::new(None, None), None, 100).unwrap();
    actor.list_profiles(request).unwrap()
}

#[test]
fn sequential_open_on_shared_profiles_db_round_trips_profile_and_intent() {
    let root = unique_root("roundtrip");
    fs::create_dir_all(&root).unwrap();
    let path = root.join(OPERATOR_PROFILES_DB_FILE);
    let _ = fs::remove_file(&path);

    let profile_id = ProfileId::from_parts(IdParts::new(1, 9001).unwrap()).unwrap();
    let intent_json = r#"{"database":"analytics","schema":"public","selected_tab":0,"tabs":[{"title":"work","sql":"SELECT 1;"}]}"#;

    {
        let writer = PersistenceActor::open(&path).unwrap();
        writer
            .create_profile(
                sample_profile(profile_id, "shared-from-cli")
                    .persistable()
                    .unwrap(),
            )
            .unwrap();
        writer
            .put_session_intent(profile_id, intent_json.to_owned())
            .unwrap();
        writer.shutdown().unwrap();
    }

    // Second client open (simulates switching CLI ↔ native on same file).
    let reader = PersistenceActor::open(&path).unwrap();
    let page = list_all(&reader);
    let rows = page.items();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id(), profile_id);
    assert_eq!(rows[0].name().as_str(), "shared-from-cli");
    assert_eq!(rows[0].engine(), Engine::PostgreSql);
    let loaded = reader.get_profile(profile_id).unwrap().expect("profile");
    assert_eq!(loaded.connection().name().as_str(), "shared-from-cli");
    let host = loaded
        .connection()
        .properties()
        .literal(ProfileProperty::Host)
        .expect("host");
    assert_eq!(host, "127.0.0.1");
    let intent = reader
        .get_session_intent(profile_id)
        .unwrap()
        .expect("session intent");
    assert_eq!(intent.intent_json, intent_json);
    reader.shutdown().unwrap();

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn production_path_never_embeds_process_id() {
    let home = Path::new("/Users/fixture");
    let path = resolve_operator_profiles_database(home, None).unwrap();
    let rendered = path.to_string_lossy();
    assert!(
        rendered.ends_with(&format!("TableRock/{OPERATOR_PROFILES_DB_FILE}")),
        "unexpected production path {rendered}"
    );
    assert!(
        !rendered.contains("state-"),
        "production path must not use process-local state-*.db: {rendered}"
    );
    assert!(
        !rendered.contains(&std::process::id().to_string()),
        "production path must not embed pid: {rendered}"
    );
}

#[test]
fn absolute_test_root_resolution_matches_default_formula() {
    let root = unique_root("env");
    let path = resolve_operator_profiles_database(Path::new("/Users/unused"), Some(&root)).unwrap();
    assert_eq!(path, root.join(OPERATOR_PROFILES_DB_FILE));
    // Env-backed default_operator_profiles_database is the production entry;
    // pure resolve is what CLI tests drive without process-wide env mutation.
    let _ = default_operator_profiles_database;
}
