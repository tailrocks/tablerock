//! Workflow-level semantic equivalence: UniFFI facade vs PersistenceActor
//! (CLI open-default store path) produce the same durable outcomes.
//!
//! Plan 021 requires shared workflows yield the same Rust outcomes through
//! both client facades. Live engine sessions stay process-local; this suite
//! covers durable profile + session-intent + history + named-param rewrite.

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use tablerock_core::{
    BoundedText, ByteLimit, Engine, FindReplaceMode, FindReplaceScope, IdParts, ProfileAggregate,
    ProfileConnectionSnapshot, ProfileDurability, ProfileId, ProfileIdentity, ProfileLimits,
    ProfileListFilter, ProfileListRequest, ProfileName, ProfileOrganization, ProfilePolicy,
    ProfilePreferences, ProfileProperty, ProfilePropertyBinding, ProfilePropertySet,
    ProfileSafetyMode, ReconnectPreference, Revision, TlsPolicy, rewrite_named_params,
};
use tablerock_ffi::{BridgeProfileDraft, BridgeSessionIntent, BridgeWorkspaceTab, TableRockBridge};
use tablerock_persistence::{
    HistoryAppend, HistoryOutcomeClass, HistoryRetention, OPERATOR_PROFILES_DB_FILE,
    PersistenceActor,
};

fn unique_db(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "tablerock-workflow-eq-{}-{}-{}",
        label,
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&root).unwrap();
    root.join(OPERATOR_PROFILES_DB_FILE)
}

fn draft(name: &str) -> BridgeProfileDraft {
    BridgeProfileDraft {
        id_bytes: None,
        revision: 0,
        engine: "postgresql".into(),
        name: name.into(),
        group: "equiv".into(),
        environment: "development".into(),
        host: "127.0.0.1".into(),
        port: "5432".into(),
        database: "app".into(),
        username: "u".into(),
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

#[test]
fn profile_and_session_intent_match_across_facades() {
    let path = unique_db("profile-intent");
    let bridge = TableRockBridge::new_for_test();
    bridge
        .configure_persistence(path.to_string_lossy().into_owned())
        .unwrap();
    let id = bridge.save_profile(draft("workflow-a")).unwrap();
    let intent = BridgeSessionIntent {
        database: "app".into(),
        schema: Some("public".into()),
        selected_tab: 0,
        tabs: vec![BridgeWorkspaceTab {
            title: "q1".into(),
            statement_text: "SELECT :id;".into(),
        }],
    };
    bridge
        .put_session_intent(id.clone(), intent.clone())
        .unwrap();
    let listed = bridge.list_profiles().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "workflow-a");
    assert_eq!(listed[0].host.as_deref(), Some("127.0.0.1"));
    let _ = bridge.shutdown(false, 1_000);

    let actor = PersistenceActor::open(&path).unwrap();
    let profile_id = ProfileId::from_bytes(id.clone().try_into().unwrap()).unwrap();
    let loaded = actor.get_profile(profile_id).unwrap().unwrap();
    assert_eq!(loaded.connection().name().as_str(), "workflow-a");
    assert_eq!(
        loaded
            .connection()
            .properties()
            .literal(ProfileProperty::Host)
            .unwrap(),
        "127.0.0.1"
    );
    let restored = actor.get_session_intent(profile_id).unwrap().unwrap();
    assert!(restored.intent_json.contains("SELECT :id;"));
    assert!(restored.intent_json.contains("\"database\":\"app\""));
    actor.shutdown().unwrap();
    let _ = fs::remove_file(&path);
}

#[test]
fn history_append_visible_to_both_facades() {
    let path = unique_db("history");
    let profile_id = ProfileId::from_parts(IdParts::new(9, 1).unwrap()).unwrap();
    {
        let actor = PersistenceActor::open(&path).unwrap();
        let properties = ProfilePropertySet::new(vec![
            ProfilePropertyBinding::literal(
                ProfileProperty::Host,
                BoundedText::copy_from_str("h", ByteLimit::new(8)).unwrap(),
            )
            .unwrap(),
            ProfilePropertyBinding::literal(
                ProfileProperty::Port,
                BoundedText::copy_from_str("1", ByteLimit::new(4)).unwrap(),
            )
            .unwrap(),
        ])
        .unwrap();
        let connection = ProfileConnectionSnapshot::new(
            ProfileIdentity::new(
                profile_id,
                Revision::INITIAL,
                Engine::PostgreSql,
                ProfileName::new(BoundedText::copy_from_str("hist", ByteLimit::new(16)).unwrap())
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
        actor
            .append_history(HistoryAppend {
                engine: Engine::PostgreSql,
                database_name: "app".into(),
                schema_name: Some("public".into()),
                statement_text: "SELECT 1".into(),
                outcome: HistoryOutcomeClass::Completed,
                retention: HistoryRetention::Full,
            })
            .unwrap();
        actor.shutdown().unwrap();
    }

    let bridge = TableRockBridge::new_for_test();
    bridge
        .configure_persistence(path.to_string_lossy().into_owned())
        .unwrap();
    let history = bridge.list_history(None, 10).unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].statement_text.as_deref(), Some("SELECT 1"));
    let _ = bridge.shutdown(false, 1_000);
    let _ = fs::remove_file(&path);
}

#[test]
fn named_param_rewrite_is_shared_rust_authority() {
    // Both clients must use core rewrite — never string-substitute values.
    let plan = rewrite_named_params("SELECT :id, :name, :id").unwrap();
    assert_eq!(plan.names, vec!["id".to_owned(), "name".to_owned()]);
    assert!(plan.sql.contains("$1"));
    assert!(plan.sql.contains("$2"));
    let bound = plan.render_with_placeholders(|index, name| {
        assert!(name == "id" || name == "name");
        format!("${}", index + 1)
    });
    assert_eq!(bound.matches("$1").count(), 2);
    assert_eq!(bound.matches("$2").count(), 1);
}

#[test]
fn find_replace_modes_match_native_contract() {
    use tablerock_core::{find_all, replace_all};
    let text = "foo food FOO";
    let words = find_all(
        text,
        "foo",
        FindReplaceMode::WholeWord,
        FindReplaceScope::Document,
        None,
    )
    .unwrap();
    assert_eq!(words.len(), 2);
    let out = replace_all(
        "a1b2",
        r"(\d)",
        "N",
        FindReplaceMode::RegularExpression,
        FindReplaceScope::Document,
        None,
    )
    .unwrap();
    assert_eq!(out.text, "aNbN");
    assert_eq!(out.count, 2);
}

#[test]
fn list_profiles_after_actor_seed_matches_bridge_shape() {
    let path = unique_db("list-shape");
    let profile_id = ProfileId::from_parts(IdParts::new(4, 88).unwrap()).unwrap();
    {
        let actor = PersistenceActor::open(&path).unwrap();
        let properties = ProfilePropertySet::new(vec![
            ProfilePropertyBinding::literal(
                ProfileProperty::Host,
                BoundedText::copy_from_str("seed.host", ByteLimit::new(32)).unwrap(),
            )
            .unwrap(),
            ProfilePropertyBinding::literal(
                ProfileProperty::Port,
                BoundedText::copy_from_str("9999", ByteLimit::new(8)).unwrap(),
            )
            .unwrap(),
            ProfilePropertyBinding::literal(
                ProfileProperty::DefaultContext,
                BoundedText::copy_from_str("ctx", ByteLimit::new(8)).unwrap(),
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
                    BoundedText::copy_from_str("seed-list", ByteLimit::new(32)).unwrap(),
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
            ProfileOrganization::new(None, vec![], true, 0, None).unwrap(),
            ProfilePreferences::new(ReconnectPreference::BoundedAutomatic, true, 250).unwrap(),
        )
        .unwrap();
        actor
            .create_profile(aggregate.persistable().unwrap())
            .unwrap();
        let page = actor
            .list_profiles(
                ProfileListRequest::new(ProfileListFilter::new(None, None), None, 10).unwrap(),
            )
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
    assert_eq!(listed[0].name, "seed-list");
    assert_eq!(listed[0].host.as_deref(), Some("seed.host"));
    assert_eq!(listed[0].port.as_deref(), Some("9999"));
    assert_eq!(listed[0].context.as_deref(), Some("ctx"));
    assert!(listed[0].favorite);
    let _ = bridge.shutdown(false, 1_000);
    let _ = fs::remove_file(&path);
}
