use std::{fs, path::PathBuf};

use tablerock_core::{
    BoundedBytes, BoundedText, ByteLimit, DangerousPlaintext, Engine, EnvironmentReference,
    IdParts, KeychainReference, OnePasswordObjectId, OnePasswordReference, OnePasswordSegment,
    PlaintextAcknowledgement, ProfileAggregate, ProfileConnectionSnapshot, ProfileDurability,
    ProfileGroupName, ProfileId, ProfileIdentity, ProfileLimits, ProfileListFilter,
    ProfileListRequest, ProfileName, ProfileOrganization, ProfilePolicy, ProfilePreferences,
    ProfileProperty, ProfilePropertyBinding, ProfilePropertySet, ProfileSafetyMode,
    ProfileSearchTerm, ProfileTag, PropertyValueSource, ReconnectPreference, Revision,
    SecretSource, SecretSourceKind, StartupAction, StartupActionSet, StartupSafetyClass, TlsPolicy,
};
use tablerock_persistence::{
    HistoryAppend, HistoryOutcomeClass, HistoryRetention, PersistenceActor, PersistenceError,
};

fn text(value: &str) -> BoundedText {
    BoundedText::copy_from_str(value, ByteLimit::new(70_000)).unwrap()
}

fn path(suffix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "tablerock-profile-create-{}-{suffix}.db",
        std::process::id(),
    ))
}

fn literal(property: ProfileProperty, value: &str) -> ProfilePropertyBinding {
    ProfilePropertyBinding::literal(property, text(value)).unwrap()
}

fn secret(property: ProfileProperty, kind: SecretSourceKind) -> ProfilePropertyBinding {
    ProfilePropertyBinding::secret(property, SecretSource::new(kind))
}

fn one_password() -> OnePasswordReference {
    OnePasswordReference::new(
        OnePasswordObjectId::parse("aaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap(),
        OnePasswordObjectId::parse("bbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap(),
        OnePasswordObjectId::parse("cccccccccccccccccccccccccc").unwrap(),
        Some(OnePasswordSegment::parse("database").unwrap()),
        OnePasswordSegment::parse("username").unwrap(),
        text("Account / Vault / Item / username"),
    )
    .unwrap()
}

fn saved_profile(engine: Engine, low_id: u64) -> ProfileAggregate {
    saved_profile_at(engine, low_id, low_id, "Saved profile")
}

fn saved_profile_at(engine: Engine, low_id: u64, revision: u64, name: &str) -> ProfileAggregate {
    let properties = ProfilePropertySet::new(vec![
        literal(ProfileProperty::Host, "database.internal"),
        literal(ProfileProperty::Port, "5432"),
        secret(
            ProfileProperty::DefaultContext,
            SecretSourceKind::HostEnvironment(EnvironmentReference::parse("TABLE_DB").unwrap()),
        ),
        secret(
            ProfileProperty::Username,
            SecretSourceKind::OnePassword(one_password()),
        ),
        secret(
            ProfileProperty::Password,
            SecretSourceKind::DangerousPlaintext(
                DangerousPlaintext::new(
                    vec![0, 255, 1],
                    PlaintextAcknowledgement::LocalTestingOnly,
                )
                .unwrap(),
            ),
        ),
        secret(
            ProfileProperty::TlsServerName,
            SecretSourceKind::Keychain(
                KeychainReference::new(
                    BoundedBytes::copy_from_slice(&[4, 5, 6], ByteLimit::new(3)).unwrap(),
                )
                .unwrap(),
            ),
        ),
        literal(
            ProfileProperty::TlsClientCertificate,
            "certificate-reference",
        ),
        secret(
            ProfileProperty::TlsClientPrivateKey,
            SecretSourceKind::PromptOnConnect,
        ),
        secret(
            ProfileProperty::TlsClientPrivateKeyPassword,
            SecretSourceKind::PromptOnConnect,
        ),
    ])
    .unwrap();
    let id = ProfileId::from_parts(IdParts::new(1, low_id).unwrap()).unwrap();
    let connection = ProfileConnectionSnapshot::new(
        ProfileIdentity::new(
            id,
            Revision::from_wire_u64(revision),
            engine,
            ProfileName::new(text(name)).unwrap(),
        ),
        properties,
        ProfilePolicy::new(
            TlsPolicy::VerifySystemRoots,
            ProfileSafetyMode::ConfirmWrites,
            ProfileLimits::new(10_000, 30_000, 5_000, 16 * 1024 * 1024).unwrap(),
        ),
    )
    .unwrap();
    ProfileAggregate::new(
        connection,
        ProfileDurability::Saved,
        ProfileOrganization::new(
            Some(ProfileGroupName::new(text("Operations")).unwrap()),
            vec![
                ProfileTag::new(text("primary")).unwrap(),
                ProfileTag::new(text("reviewed")).unwrap(),
            ],
            true,
            low_id as u32,
            None,
        )
        .unwrap(),
        ProfilePreferences::new(ReconnectPreference::BoundedAutomatic, true, 250).unwrap(),
    )
    .unwrap()
}

fn profile_id(low_id: u64) -> ProfileId {
    ProfileId::from_parts(IdParts::new(1, low_id).unwrap()).unwrap()
}

#[test]
fn startup_actions_and_ssh_properties_round_trip() {
    let path = path("startup-ssh");
    let _ = fs::remove_file(&path);
    let actor = PersistenceActor::open(&path).unwrap();
    let properties = ProfilePropertySet::new(vec![
        literal(ProfileProperty::Host, "db.internal"),
        literal(ProfileProperty::Port, "5432"),
        literal(ProfileProperty::SshHost, "bastion.internal"),
        literal(ProfileProperty::SshPort, "22"),
        literal(ProfileProperty::SshUsername, "tunnel"),
        literal(ProfileProperty::SshKnownHostsPath, "/var/lib/known_hosts"),
        secret(
            ProfileProperty::SshPassword,
            SecretSourceKind::PromptOnConnect,
        ),
    ])
    .unwrap();
    let id = profile_id(42);
    let connection = ProfileConnectionSnapshot::new(
        ProfileIdentity::new(
            id,
            Revision::INITIAL,
            Engine::PostgreSql,
            ProfileName::new(text("SSH profile")).unwrap(),
        ),
        properties,
        ProfilePolicy::new(
            TlsPolicy::Disabled,
            ProfileSafetyMode::ReadOnly,
            ProfileLimits::new(10_000, 30_000, 5_000, 16 * 1024 * 1024).unwrap(),
        ),
    )
    .unwrap();
    let startup = StartupActionSet::new(vec![
        StartupAction::from_str("SELECT 1", StartupSafetyClass::ReadOnly, 5_000, true).unwrap(),
        StartupAction::from_str(
            "SET application_name = 'tablerock'",
            StartupSafetyClass::ReadOnly,
            2_000,
            false,
        )
        .unwrap(),
    ])
    .unwrap();
    let profile = ProfileAggregate::new(
        connection,
        ProfileDurability::Saved,
        ProfileOrganization::new(None, Vec::new(), false, 0, None).unwrap(),
        ProfilePreferences::new(ReconnectPreference::Manual, false, 100).unwrap(),
    )
    .unwrap()
    .with_startup_actions(startup);
    actor
        .create_profile(profile.persistable().unwrap())
        .unwrap();
    let loaded = actor.get_profile(id).unwrap().unwrap();
    assert_eq!(
        loaded
            .connection()
            .properties()
            .literal(ProfileProperty::SshHost),
        Some("bastion.internal")
    );
    assert_eq!(loaded.startup_actions().len(), 2);
    assert_eq!(
        loaded.startup_actions().actions()[0].statement(),
        "SELECT 1"
    );
    assert!(!loaded.startup_actions().actions()[1].run_on_reconnect());
    assert!(!format!("{loaded:?}").contains("SELECT 1"));
    actor.shutdown().unwrap();
    fs::remove_file(&path).unwrap();
}

#[test]
fn ssh_use_agent_preference_round_trip() {
    let path = path("ssh-agent-pref");
    let _ = fs::remove_file(&path);
    let actor = PersistenceActor::open(&path).unwrap();
    let properties = ProfilePropertySet::new(vec![
        literal(ProfileProperty::Host, "db.internal"),
        literal(ProfileProperty::Port, "5432"),
    ])
    .unwrap();
    let id = profile_id(88);
    let connection = ProfileConnectionSnapshot::new(
        ProfileIdentity::new(
            id,
            Revision::INITIAL,
            Engine::PostgreSql,
            ProfileName::new(text("Agent profile")).unwrap(),
        ),
        properties,
        ProfilePolicy::new(
            TlsPolicy::Disabled,
            ProfileSafetyMode::ReadOnly,
            ProfileLimits::new(10_000, 30_000, 5_000, 16 * 1024 * 1024).unwrap(),
        ),
    )
    .unwrap();
    let profile = ProfileAggregate::new(
        connection,
        ProfileDurability::Saved,
        ProfileOrganization::new(None, Vec::new(), false, 0, None).unwrap(),
        ProfilePreferences::new(ReconnectPreference::Manual, false, 100)
            .unwrap()
            .with_ssh_use_agent(true),
    )
    .unwrap();
    assert!(profile.preferences().ssh_use_agent());
    actor
        .create_profile(profile.persistable().unwrap())
        .unwrap();
    let loaded = actor.get_profile(id).unwrap().unwrap();
    assert!(loaded.preferences().ssh_use_agent());
    actor.shutdown().unwrap();
    fs::remove_file(&path).unwrap();
}

fn search(value: &str) -> ProfileSearchTerm {
    ProfileSearchTerm::new(text(value)).unwrap()
}

async fn count(connection: &turso::Connection, sql: &str) -> u32 {
    let mut rows = connection.query(sql, ()).await.unwrap();
    rows.next().await.unwrap().unwrap().get::<u32>(0).unwrap()
}

#[test]
fn saved_token_creates_complete_rows_atomically_for_every_engine() {
    let path = path("success");
    let _ = fs::remove_file(&path);
    let actor = PersistenceActor::open(&path).unwrap();
    assert_eq!(actor.get_profile(profile_id(99)).unwrap(), None);
    for (index, engine) in [Engine::PostgreSql, Engine::ClickHouse, Engine::Redis]
        .into_iter()
        .enumerate()
    {
        let profile = saved_profile(engine, index as u64 + 1);
        actor
            .create_profile(profile.persistable().unwrap())
            .unwrap();
        let loaded = actor
            .get_profile(profile.connection().id())
            .unwrap()
            .unwrap();
        assert_eq!(loaded, profile);
        let debug = format!("{loaded:?}");
        for sensitive in [
            "database.internal",
            "Saved profile",
            "Operations",
            "TABLE_DB",
            "aaaaaaaaaaaaaaaaaaaaaaaaaa",
            "Account / Vault / Item / username",
        ] {
            assert!(!debug.contains(sensitive));
        }
        assert!(matches!(
            actor.create_profile(profile.persistable().unwrap()),
            Err(PersistenceError::ProfileAlreadyExists)
        ));
    }
    actor.shutdown().unwrap();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let database = turso::Builder::new_local(path.to_str().unwrap())
            .build()
            .await
            .unwrap();
        let connection = database.connect().unwrap();
        assert_eq!(
            count(&connection, "SELECT COUNT(*) FROM saved_profiles").await,
            3
        );
        assert_eq!(
            count(&connection, "SELECT COUNT(*) FROM saved_profile_tags").await,
            6
        );
        assert_eq!(
            count(&connection, "SELECT COUNT(*) FROM saved_profile_properties").await,
            27
        );
        assert_eq!(
            count(
                &connection,
                "SELECT COUNT(*) FROM saved_profile_properties \
                 WHERE source_kind = 6 AND length(blob_value) = 3",
            )
            .await,
            3
        );
    });

    let reopened = PersistenceActor::open(&path).unwrap();
    assert_eq!(reopened.health().unwrap().schema_version, 15);
    for (index, engine) in [Engine::PostgreSql, Engine::ClickHouse, Engine::Redis]
        .into_iter()
        .enumerate()
    {
        let expected = saved_profile(engine, index as u64 + 1);
        assert_eq!(
            reopened.get_profile(expected.connection().id()).unwrap(),
            Some(expected)
        );
    }
    reopened.shutdown().unwrap();
    fs::remove_file(path).unwrap();
}

#[test]
fn malformed_saved_value_fails_closed_and_rolls_back_read_transaction() {
    let path = path("malformed-read");
    let _ = fs::remove_file(&path);
    let actor = PersistenceActor::open(&path).unwrap();
    let profile = saved_profile(Engine::PostgreSql, 7);
    actor
        .create_profile(profile.persistable().unwrap())
        .unwrap();
    actor.shutdown().unwrap();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let database = turso::Builder::new_local(path.to_str().unwrap())
            .build()
            .await
            .unwrap();
        let connection = database.connect().unwrap();
        connection
            .execute(
                "UPDATE saved_profiles SET name = '   ' WHERE profile_id = ?1",
                (profile.connection().id().to_bytes().as_slice(),),
            )
            .await
            .unwrap();
    });

    let actor = PersistenceActor::open(&path).unwrap();
    assert_eq!(
        actor
            .list_profiles(
                ProfileListRequest::new(ProfileListFilter::default(), None, 10).unwrap(),
            )
            .unwrap_err(),
        PersistenceError::ProfileDecode
    );
    let error = actor.get_profile(profile.connection().id()).unwrap_err();
    assert_eq!(error, PersistenceError::ProfileDecode);
    assert_eq!(format!("{error:?}"), "ProfileDecode");
    assert_eq!(error.to_string(), "local persistence operation failed");
    assert_eq!(actor.health().unwrap().schema_version, 15);
    actor.shutdown().unwrap();
    fs::remove_file(path).unwrap();
}

#[test]
fn malformed_literal_endpoint_fails_closed_in_summary_projection() {
    let path = path("malformed-endpoint");
    let _ = fs::remove_file(&path);
    let actor = PersistenceActor::open(&path).unwrap();
    let profile = saved_profile(Engine::Redis, 8);
    actor
        .create_profile(profile.persistable().unwrap())
        .unwrap();
    actor.shutdown().unwrap();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let database = turso::Builder::new_local(path.to_str().unwrap())
            .build()
            .await
            .unwrap();
        let connection = database.connect().unwrap();
        connection
            .execute(
                "UPDATE saved_profile_properties SET text_value = '99999' \
                 WHERE profile_id = ?1 AND property = 2",
                (profile_id(8).to_bytes().as_slice(),),
            )
            .await
            .unwrap();
    });

    let actor = PersistenceActor::open(&path).unwrap();
    assert_eq!(
        actor
            .list_profiles(
                ProfileListRequest::new(ProfileListFilter::default(), None, 10).unwrap(),
            )
            .unwrap_err(),
        PersistenceError::ProfileDecode
    );
    assert_eq!(actor.health().unwrap().schema_version, 15);
    actor.shutdown().unwrap();
    fs::remove_file(path).unwrap();
}

#[test]
fn bounded_profile_list_uses_stable_keyset_order_without_secret_payloads() {
    let path = path("list");
    let _ = fs::remove_file(&path);
    let actor = PersistenceActor::open(&path).unwrap();
    for (engine, low) in [
        (Engine::PostgreSql, 60),
        (Engine::ClickHouse, 61),
        (Engine::Redis, 62),
    ] {
        let profile = saved_profile_at(engine, low, low, "Private profile name");
        actor
            .create_profile(profile.persistable().unwrap())
            .unwrap();
    }
    actor.shutdown().unwrap();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let database = turso::Builder::new_local(path.to_str().unwrap())
            .build()
            .await
            .unwrap();
        let connection = database.connect().unwrap();
        connection
            .execute(
                "UPDATE saved_profiles SET favorite = 0 WHERE profile_id = ?1",
                (profile_id(60).to_bytes().as_slice(),),
            )
            .await
            .unwrap();
        connection
            .execute(
                "UPDATE saved_profiles SET group_name = 'Analytics' WHERE profile_id = ?1",
                (profile_id(61).to_bytes().as_slice(),),
            )
            .await
            .unwrap();
        connection
            .execute(
                "UPDATE saved_profile_tags SET tag = 'cache' \
                 WHERE profile_id = ?1 AND ordinal = 0",
                (profile_id(62).to_bytes().as_slice(),),
            )
            .await
            .unwrap();
        connection
            .execute(
                "UPDATE saved_profile_properties \
                 SET source_kind = 4, source_schema = 1, text_value = 'SECRET_HOST_REFERENCE' \
                 WHERE profile_id = ?1 AND property = 1",
                (profile_id(62).to_bytes().as_slice(),),
            )
            .await
            .unwrap();
        let mut plan = connection
            .query(
                "EXPLAIN QUERY PLAN SELECT profile_id FROM saved_profiles \
                 ORDER BY favorite DESC, saved_order, profile_id LIMIT 3",
                (),
            )
            .await
            .unwrap();
        let mut uses_list_index = false;
        while let Some(row) = plan.next().await.unwrap() {
            uses_list_index |= row
                .get::<String>(3)
                .unwrap()
                .contains("saved_profiles_bounded_list");
        }
        assert!(uses_list_index);
        let mut engine_plan = connection
            .query(
                "EXPLAIN QUERY PLAN SELECT profile_id FROM saved_profiles WHERE engine = 2 \
                 ORDER BY favorite DESC, saved_order, profile_id LIMIT 3",
                (),
            )
            .await
            .unwrap();
        let mut uses_engine_index = false;
        while let Some(row) = engine_plan.next().await.unwrap() {
            uses_engine_index |= row
                .get::<String>(3)
                .unwrap()
                .contains("saved_profiles_engine_bounded_list");
        }
        assert!(uses_engine_index);
        let mut group_plan = connection
            .query(
                "EXPLAIN QUERY PLAN SELECT profile_id FROM saved_profiles \
                 WHERE group_name = 'Operations' \
                 ORDER BY favorite DESC, saved_order, profile_id LIMIT 3",
                (),
            )
            .await
            .unwrap();
        let mut uses_group_index = false;
        while let Some(row) = group_plan.next().await.unwrap() {
            uses_group_index |= row
                .get::<String>(3)
                .unwrap()
                .contains("saved_profiles_group_bounded_list");
        }
        assert!(uses_group_index);
        let mut tag_plan = connection
            .query(
                "EXPLAIN QUERY PLAN SELECT saved_profiles.profile_id \
                 FROM saved_profile_tags filtered_tag \
                 JOIN saved_profiles ON saved_profiles.profile_id = filtered_tag.profile_id \
                 WHERE filtered_tag.tag = 'cache' \
                 ORDER BY favorite DESC, saved_order, saved_profiles.profile_id LIMIT 3",
                (),
            )
            .await
            .unwrap();
        let mut uses_tag_index = false;
        while let Some(row) = tag_plan.next().await.unwrap() {
            uses_tag_index |= row
                .get::<String>(3)
                .unwrap()
                .contains("saved_profile_tags_lookup");
        }
        assert!(uses_tag_index);
    });

    let actor = PersistenceActor::open(&path).unwrap();
    let first = actor
        .list_profiles(ProfileListRequest::new(ProfileListFilter::default(), None, 2).unwrap())
        .unwrap();
    assert_eq!(
        first
            .items()
            .iter()
            .map(|item| item.id())
            .collect::<Vec<_>>(),
        vec![profile_id(61), profile_id(62)]
    );
    for item in first.items() {
        assert_eq!(item.name().as_str(), "Private profile name");
        assert_eq!(
            item.group().unwrap().as_str(),
            if item.id() == profile_id(61) {
                "Analytics"
            } else {
                "Operations"
            }
        );
        assert_eq!(
            item.endpoint().host().source(),
            if item.id() == profile_id(62) {
                PropertyValueSource::SecretSource
            } else {
                PropertyValueSource::Literal
            }
        );
        assert_eq!(
            item.endpoint().host().literal_value(),
            (item.id() != profile_id(62)).then_some("database.internal")
        );
        assert_eq!(
            item.endpoint().port().source(),
            PropertyValueSource::Literal
        );
        assert_eq!(item.endpoint().port().literal_value(), Some("5432"));
        assert!(item.sources().has_secret_sources());
        assert!(item.sources().has_dangerous_plaintext());
        let debug = format!("{item:?}");
        for sensitive in [
            "Private profile name",
            "Operations",
            "database.internal",
            "TABLE_DB",
            "aaaaaaaaaaaaaaaaaaaaaaaaaa",
            "SECRET_HOST_REFERENCE",
        ] {
            assert!(!debug.contains(sensitive));
        }
    }
    let second = actor
        .list_profiles(
            ProfileListRequest::new(ProfileListFilter::default(), first.next(), 2).unwrap(),
        )
        .unwrap();
    assert_eq!(second.items().len(), 1);
    assert_eq!(second.items()[0].id(), profile_id(60));
    assert!(!second.items()[0].favorite());
    assert_eq!(second.next(), None);

    let favorite_filter = ProfileListFilter::new(None, Some(true));
    let favorites = actor
        .list_profiles(ProfileListRequest::new(favorite_filter.clone(), None, 1).unwrap())
        .unwrap();
    assert_eq!(favorites.items()[0].id(), profile_id(61));
    let remaining_favorites = actor
        .list_profiles(ProfileListRequest::new(favorite_filter, favorites.next(), 10).unwrap())
        .unwrap();
    assert_eq!(remaining_favorites.items().len(), 1);
    assert_eq!(remaining_favorites.items()[0].id(), profile_id(62));

    let non_favorites = actor
        .list_profiles(
            ProfileListRequest::new(ProfileListFilter::new(None, Some(false)), None, 10).unwrap(),
        )
        .unwrap();
    assert_eq!(non_favorites.items().len(), 1);
    assert_eq!(non_favorites.items()[0].id(), profile_id(60));

    for (engine, expected) in [
        (Engine::PostgreSql, profile_id(60)),
        (Engine::ClickHouse, profile_id(61)),
        (Engine::Redis, profile_id(62)),
    ] {
        let page = actor
            .list_profiles(
                ProfileListRequest::new(ProfileListFilter::new(Some(engine), None), None, 10)
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(page.items().len(), 1);
        assert_eq!(page.items()[0].id(), expected);
    }
    let impossible = actor
        .list_profiles(
            ProfileListRequest::new(
                ProfileListFilter::new(Some(Engine::ClickHouse), Some(false)),
                None,
                10,
            )
            .unwrap(),
        )
        .unwrap();
    assert!(impossible.items().is_empty());

    let operations = actor
        .list_profiles(
            ProfileListRequest::new(
                ProfileListFilter::default()
                    .with_group(Some(ProfileGroupName::new(text("Operations")).unwrap())),
                None,
                10,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(
        operations
            .items()
            .iter()
            .map(|item| item.id())
            .collect::<Vec<_>>(),
        vec![profile_id(62), profile_id(60)]
    );
    let analytics = actor
        .list_profiles(
            ProfileListRequest::new(
                ProfileListFilter::default()
                    .with_group(Some(ProfileGroupName::new(text("Analytics")).unwrap())),
                None,
                10,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(analytics.items().len(), 1);
    assert_eq!(analytics.items()[0].id(), profile_id(61));

    let cache = actor
        .list_profiles(
            ProfileListRequest::new(
                ProfileListFilter::default()
                    .with_tag(Some(ProfileTag::new(text("cache")).unwrap())),
                None,
                10,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(cache.items().len(), 1);
    assert_eq!(cache.items()[0].id(), profile_id(62));
    let combined = actor
        .list_profiles(
            ProfileListRequest::new(
                ProfileListFilter::new(Some(Engine::Redis), Some(true))
                    .with_group(Some(ProfileGroupName::new(text("Operations")).unwrap()))
                    .with_tag(Some(ProfileTag::new(text("cache")).unwrap())),
                None,
                10,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(combined.items().len(), 1);
    assert_eq!(combined.items()[0].id(), profile_id(62));

    let search_filter = ProfileListFilter::default().with_search(Some(search("PRIVATE")));
    let search_first = actor
        .list_profiles(ProfileListRequest::new(search_filter.clone(), None, 1).unwrap())
        .unwrap();
    assert_eq!(search_first.items()[0].id(), profile_id(61));
    let search_second = actor
        .list_profiles(
            ProfileListRequest::new(search_filter.clone(), search_first.next(), 1).unwrap(),
        )
        .unwrap();
    assert_eq!(search_second.items()[0].id(), profile_id(62));
    let search_final = actor
        .list_profiles(ProfileListRequest::new(search_filter, search_second.next(), 1).unwrap())
        .unwrap();
    assert_eq!(search_final.items()[0].id(), profile_id(60));
    assert_eq!(search_final.next(), None);

    let group_search = actor
        .list_profiles(
            ProfileListRequest::new(
                ProfileListFilter::default().with_search(Some(search("ANALYTICS"))),
                None,
                10,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(group_search.items().len(), 1);
    assert_eq!(group_search.items()[0].id(), profile_id(61));
    let tag_search = actor
        .list_profiles(
            ProfileListRequest::new(
                ProfileListFilter::new(Some(Engine::Redis), Some(true))
                    .with_search(Some(search("CACHE"))),
                None,
                10,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(tag_search.items().len(), 1);
    assert_eq!(tag_search.items()[0].id(), profile_id(62));
    let no_match = actor
        .list_profiles(
            ProfileListRequest::new(
                ProfileListFilter::default().with_search(Some(search("absent"))),
                None,
                10,
            )
            .unwrap(),
        )
        .unwrap();
    assert!(no_match.items().is_empty());
    actor.shutdown().unwrap();
    fs::remove_file(path).unwrap();
}

#[test]
fn replacement_uses_atomic_revision_compare_and_swap() {
    let path = path("replace");
    let _ = fs::remove_file(&path);
    let actor = PersistenceActor::open(&path).unwrap();
    let current = saved_profile_at(Engine::Redis, 20, 20, "Current");
    actor
        .create_profile(current.persistable().unwrap())
        .unwrap();

    let replacement = saved_profile_at(Engine::Redis, 20, 21, "Replacement");
    actor
        .replace_profile(
            Revision::from_wire_u64(20),
            replacement.persistable().unwrap(),
        )
        .unwrap();
    assert_eq!(
        actor.get_profile(profile_id(20)).unwrap(),
        Some(replacement)
    );

    let stale = saved_profile_at(Engine::Redis, 20, 21, "Stale");
    assert_eq!(
        actor
            .replace_profile(Revision::from_wire_u64(20), stale.persistable().unwrap(),)
            .unwrap_err(),
        PersistenceError::ProfileStaleRevision
    );
    let skipped = saved_profile_at(Engine::Redis, 20, 23, "Skipped");
    assert_eq!(
        actor
            .replace_profile(Revision::from_wire_u64(21), skipped.persistable().unwrap(),)
            .unwrap_err(),
        PersistenceError::ProfileInvalidRevision
    );
    let missing = saved_profile_at(Engine::PostgreSql, 30, 1, "Missing");
    assert_eq!(
        actor
            .replace_profile(Revision::INITIAL, missing.persistable().unwrap(),)
            .unwrap_err(),
        PersistenceError::ProfileNotFound
    );
    assert_eq!(
        actor
            .get_profile(profile_id(20))
            .unwrap()
            .unwrap()
            .connection()
            .revision(),
        Revision::from_wire_u64(21)
    );
    actor.shutdown().unwrap();
    fs::remove_file(path).unwrap();
}

#[test]
fn replacement_child_failure_preserves_previous_aggregate() {
    let path = path("replace-rollback");
    let _ = fs::remove_file(&path);
    let actor = PersistenceActor::open(&path).unwrap();
    let current = saved_profile_at(Engine::ClickHouse, 40, 40, "Current");
    actor
        .create_profile(current.persistable().unwrap())
        .unwrap();
    actor.shutdown().unwrap();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let database = turso::Builder::new_local(path.to_str().unwrap())
            .build()
            .await
            .unwrap();
        let connection = database.connect().unwrap();
        connection
            .execute_batch(
                "CREATE TRIGGER reject_replacement_tags \
                 BEFORE INSERT ON saved_profile_tags \
                 BEGIN SELECT RAISE(ABORT, 'injected replacement failure'); END;",
            )
            .await
            .unwrap();
    });

    let actor = PersistenceActor::open(&path).unwrap();
    let replacement = saved_profile_at(Engine::ClickHouse, 40, 41, "Replacement");
    assert_eq!(
        actor
            .replace_profile(
                Revision::from_wire_u64(40),
                replacement.persistable().unwrap(),
            )
            .unwrap_err(),
        PersistenceError::ProfileWrite
    );
    assert_eq!(actor.get_profile(profile_id(40)).unwrap(), Some(current));
    actor.shutdown().unwrap();
    fs::remove_file(path).unwrap();
}

#[test]
fn deletion_uses_revision_compare_and_swap_and_removes_only_owned_rows() {
    let path = path("delete");
    let _ = fs::remove_file(&path);
    let actor = PersistenceActor::open(&path).unwrap();
    let profile = saved_profile_at(Engine::PostgreSql, 50, 8, "Delete me");
    actor
        .create_profile(profile.persistable().unwrap())
        .unwrap();
    actor
        .append_history(HistoryAppend {
            engine: Engine::PostgreSql,
            database_name: "postgres".into(),
            schema_name: Some("public".into()),
            statement_text: "SELECT retained_after_profile_delete".into(),
            outcome: HistoryOutcomeClass::Completed,
            retention: HistoryRetention::Full,
        })
        .unwrap()
        .unwrap();

    assert_eq!(
        actor
            .delete_profile(profile_id(50), Revision::from_wire_u64(7))
            .unwrap_err(),
        PersistenceError::ProfileStaleRevision
    );
    assert_eq!(actor.get_profile(profile_id(50)).unwrap(), Some(profile));
    assert_eq!(
        actor
            .delete_profile(profile_id(99), Revision::INITIAL)
            .unwrap_err(),
        PersistenceError::ProfileNotFound
    );
    actor
        .delete_profile(profile_id(50), Revision::from_wire_u64(8))
        .unwrap();
    assert_eq!(actor.get_profile(profile_id(50)).unwrap(), None);
    assert_eq!(actor.history_count().unwrap(), 1);
    assert_eq!(
        actor.list_history(None, 10).unwrap()[0]
            .statement_text
            .as_deref(),
        Some("SELECT retained_after_profile_delete")
    );
    assert_eq!(
        actor
            .delete_profile(profile_id(50), Revision::from_wire_u64(8))
            .unwrap_err(),
        PersistenceError::ProfileNotFound
    );
    actor.shutdown().unwrap();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let database = turso::Builder::new_local(path.to_str().unwrap())
            .build()
            .await
            .unwrap();
        let connection = database.connect().unwrap();
        assert_eq!(
            count(&connection, "SELECT COUNT(*) FROM saved_profiles").await,
            0
        );
        assert_eq!(
            count(&connection, "SELECT COUNT(*) FROM saved_profile_tags").await,
            0
        );
        assert_eq!(
            count(&connection, "SELECT COUNT(*) FROM saved_profile_properties",).await,
            0
        );
    });

    let reopened = PersistenceActor::open(&path).unwrap();
    assert_eq!(reopened.get_profile(profile_id(50)).unwrap(), None);
    reopened.shutdown().unwrap();
    fs::remove_file(path).unwrap();
}

#[test]
fn child_row_failure_rolls_back_the_entire_profile() {
    let path = path("rollback");
    let _ = fs::remove_file(&path);
    let actor = PersistenceActor::open(&path).unwrap();
    actor.shutdown().unwrap();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let database = turso::Builder::new_local(path.to_str().unwrap())
            .build()
            .await
            .unwrap();
        let connection = database.connect().unwrap();
        connection
            .execute_batch(
                "CREATE TRIGGER reject_profile_tags \
                 BEFORE INSERT ON saved_profile_tags \
                 BEGIN SELECT RAISE(ABORT, 'injected tag failure'); END;",
            )
            .await
            .unwrap();
    });

    let actor = PersistenceActor::open(&path).unwrap();
    let profile = saved_profile(Engine::PostgreSql, 9);
    assert!(matches!(
        actor.create_profile(profile.persistable().unwrap()),
        Err(PersistenceError::ProfileWrite)
    ));
    actor.shutdown().unwrap();

    runtime.block_on(async {
        let database = turso::Builder::new_local(path.to_str().unwrap())
            .build()
            .await
            .unwrap();
        let connection = database.connect().unwrap();
        assert_eq!(
            count(&connection, "SELECT COUNT(*) FROM saved_profiles").await,
            0
        );
        assert_eq!(
            count(&connection, "SELECT COUNT(*) FROM saved_profile_properties").await,
            0
        );
    });
    fs::remove_file(path).unwrap();
}
