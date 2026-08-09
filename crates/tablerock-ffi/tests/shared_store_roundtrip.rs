//! Cross-client durable round-trip: UniFFI write ↔ PersistenceActor (CLI path) read.
//!
//! Uses one real `profiles.db` file. Live sessions stay process-local; this
//! proves shared durable profiles + session intent for client switching.

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use tablerock_core::{
    BoundedText, ByteLimit, Engine, IdParts, ProfileAggregate, ProfileConnectionSnapshot,
    ProfileDurability, ProfileGroupName, ProfileId, ProfileIdentity, ProfileLimits,
    ProfileListFilter, ProfileListRequest, ProfileName, ProfileOrganization, ProfilePolicy,
    ProfilePreferences, ProfileProperty, ProfilePropertyBinding, ProfilePropertySet,
    ProfileSafetyMode, ProfileTag, ReconnectPreference, Revision, TlsPolicy,
};
use tablerock_ffi::{
    BridgeProfileDraft, BridgeSessionIntent, BridgeWorkspaceTab, TableRockBridge,
};
use tablerock_persistence::{
    OPERATOR_PROFILES_DB_FILE, PersistenceActor, resolve_operator_profiles_database,
};

fn unique_db(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "tablerock-cross-client-{}-{}-{}",
        label,
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&root).unwrap();
    root.join(OPERATOR_PROFILES_DB_FILE)
}

fn empty_draft(name: &str) -> BridgeProfileDraft {
    BridgeProfileDraft {
        id_bytes: None,
        revision: 0,
        engine: "postgresql".into(),
        name: name.into(),
        group: "shared".into(),
        environment: "development".into(),
        host: "db.example".into(),
        port: "5432".into(),
        database: "analytics".into(),
        username: "reader".into(),
        password_source: "prompt".into(),
        password_value: String::new(),
        password_reference: None,
        has_stored_password: false,
        plaintext_acknowledged: false,
        tls_mode: "off".into(),
        safety_mode: "confirm_writes".into(),
        ssh_enabled: false,
        ssh_host: String::new(),
        ssh_port: String::new(),
        ssh_username: String::new(),
        ssh_auth_mode: "agent".into(),
        ssh_password: String::new(),
        ssh_private_key: String::new(),
        ssh_known_hosts_path: String::new(),
        ssh_has_stored_password: false,
        ssh_has_stored_private_key: false,
        ssh_plaintext_acknowledged: false,
        startup_actions: vec![],
    }
}

fn sample_intent() -> BridgeSessionIntent {
    BridgeSessionIntent {
        database: "analytics".into(),
        schema: Some("public".into()),
        selected_tab: 0,
        tabs: vec![BridgeWorkspaceTab {
            title: "restore-me".into(),
            statement_text: "SELECT current_database();".into(),
        }],
    }
}

fn sample_aggregate(id: ProfileId, name: &str) -> ProfileAggregate {
    let properties = ProfilePropertySet::new(vec![
        ProfilePropertyBinding::literal(
            ProfileProperty::Host,
            BoundedText::copy_from_str("cli.example", ByteLimit::new(32)).unwrap(),
        )
        .unwrap(),
        ProfilePropertyBinding::literal(
            ProfileProperty::Port,
            BoundedText::copy_from_str("6432", ByteLimit::new(8)).unwrap(),
        )
        .unwrap(),
        ProfilePropertyBinding::literal(
            ProfileProperty::DefaultContext,
            BoundedText::copy_from_str("warehouse", ByteLimit::new(32)).unwrap(),
        )
        .unwrap(),
        ProfilePropertyBinding::literal(
            ProfileProperty::Username,
            BoundedText::copy_from_str("cli-user", ByteLimit::new(32)).unwrap(),
        )
        .unwrap(),
    ])
    .unwrap();
    let connection = ProfileConnectionSnapshot::new(
        ProfileIdentity::new(
            id,
            Revision::INITIAL,
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
                    BoundedText::copy_from_str("from-cli", ByteLimit::new(16)).unwrap(),
                )
                .unwrap(),
            ),
            vec![
                ProfileTag::new(BoundedText::copy_from_str("parity", ByteLimit::new(16)).unwrap())
                    .unwrap(),
            ],
            true,
            0,
            None,
        )
        .unwrap(),
        ProfilePreferences::new(ReconnectPreference::BoundedAutomatic, true, 250).unwrap(),
    )
    .unwrap()
}

#[test]
fn uniffi_write_then_persistence_actor_read_shares_profile_and_intent() {
    let path = unique_db("ffi-then-actor");
    let _ = fs::remove_file(&path);

    let bridge = TableRockBridge::new_for_test();
    bridge
        .configure_persistence(path.to_string_lossy().into_owned())
        .unwrap();
    let id_bytes = bridge.save_profile(empty_draft("from-native")).unwrap();
    let intent = sample_intent();
    bridge
        .put_session_intent(id_bytes.clone(), intent.clone())
        .unwrap();
    let listed = bridge.list_profiles().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "from-native");
    assert_eq!(listed[0].engine, "postgresql");
    assert_eq!(listed[0].host.as_deref(), Some("db.example"));
    assert_eq!(listed[0].port.as_deref(), Some("5432"));
    assert_eq!(listed[0].context.as_deref(), Some("analytics"));
    let _ = bridge.shutdown(false, 1_000);

    let actor = PersistenceActor::open(&path).unwrap();
    let profile_id = ProfileId::from_bytes(id_bytes.clone().try_into().unwrap()).unwrap();
    let loaded = actor.get_profile(profile_id).unwrap().expect("profile");
    assert_eq!(loaded.connection().name().as_str(), "from-native");
    assert_eq!(loaded.connection().engine(), Engine::PostgreSql);
    let host = loaded
        .connection()
        .properties()
        .literal(ProfileProperty::Host)
        .expect("host");
    assert_eq!(host, "db.example");
    let restored = actor
        .get_session_intent(profile_id)
        .unwrap()
        .expect("intent");
    assert!(restored.intent_json.contains("\"database\":\"analytics\""));
    assert!(restored.intent_json.contains("restore-me"));
    assert!(restored.intent_json.contains("SELECT current_database();"));
    actor.shutdown().unwrap();
    let _ = fs::remove_file(&path);
    if let Some(parent) = path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
}

#[test]
fn persistence_actor_write_then_uniffi_read_shares_profile_and_intent() {
    let path = unique_db("actor-then-ffi");
    let _ = fs::remove_file(&path);

    let profile_id = ProfileId::from_parts(IdParts::new(2, 4242).unwrap()).unwrap();
    let intent_json = r#"{"database":"warehouse","schema":null,"selected_tab":0,"tabs":[{"title":"cli-tab","sql":"SELECT 42;"}]}"#;
    {
        let actor = PersistenceActor::open(&path).unwrap();
        let aggregate = sample_aggregate(profile_id, "from-cli-tui");
        actor
            .create_profile(aggregate.persistable().expect("saved"))
            .unwrap();
        actor
            .put_session_intent(profile_id, intent_json.to_owned())
            .unwrap();
        let page = actor
            .list_profiles(ProfileListRequest::new(ProfileListFilter::new(None, None), None, 10).unwrap())
            .unwrap();
        assert_eq!(page.items().len(), 1);
        actor.shutdown().unwrap();
    }

    let bridge = TableRockBridge::new_for_test();
    bridge
        .configure_persistence(path.to_string_lossy().into_owned())
        .unwrap();
    let listed = bridge.list_profiles().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id_bytes, profile_id.to_bytes());
    assert_eq!(listed[0].name, "from-cli-tui");
    assert_eq!(listed[0].engine, "postgresql");
    assert_eq!(listed[0].host.as_deref(), Some("cli.example"));
    assert_eq!(listed[0].port.as_deref(), Some("6432"));
    assert_eq!(listed[0].context.as_deref(), Some("warehouse"));
    assert_eq!(listed[0].group.as_deref(), Some("from-cli"));
    let intent = bridge
        .get_session_intent(profile_id.to_bytes().to_vec())
        .unwrap()
        .expect("session intent");
    assert_eq!(intent.database, "warehouse");
    assert_eq!(intent.schema, None);
    assert_eq!(intent.selected_tab, 0);
    assert_eq!(intent.tabs.len(), 1);
    assert_eq!(intent.tabs[0].title, "cli-tab");
    assert_eq!(intent.tabs[0].statement_text, "SELECT 42;");
    let _ = bridge.shutdown(false, 1_000);
    let _ = fs::remove_file(&path);
    if let Some(parent) = path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
}

#[test]
fn shared_path_formula_matches_native_application_support_layout() {
    let path = resolve_operator_profiles_database(std::path::Path::new("/Users/test"), None)
        .expect("resolve");
    #[cfg(target_os = "macos")]
    assert_eq!(
        path.to_string_lossy(),
        "/Users/test/Library/Application Support/TableRock/profiles.db"
    );
    assert_eq!(
        path.file_name().and_then(|n| n.to_str()),
        Some(OPERATOR_PROFILES_DB_FILE)
    );
    assert!(!path.to_string_lossy().contains("state-"));
}
