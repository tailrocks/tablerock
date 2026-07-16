use std::{fs, path::PathBuf};

use tablerock_core::{
    BoundedBytes, BoundedText, ByteLimit, DangerousPlaintext, Engine, EnvironmentReference,
    IdParts, KeychainReference, OnePasswordObjectId, OnePasswordReference, OnePasswordSegment,
    PlaintextAcknowledgement, ProfileAggregate, ProfileConnectionSnapshot, ProfileDurability,
    ProfileGroupName, ProfileId, ProfileIdentity, ProfileLimits, ProfileName, ProfileOrganization,
    ProfilePolicy, ProfilePreferences, ProfileProperty, ProfilePropertyBinding, ProfilePropertySet,
    ProfileSafetyMode, ProfileTag, ReconnectPreference, Revision, SecretSource, SecretSourceKind,
    TlsPolicy,
};
use tablerock_persistence::{PersistenceActor, PersistenceError};

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
            Revision::from_wire_u64(low_id),
            engine,
            ProfileName::new(text("Saved profile")).unwrap(),
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
        )
        .unwrap(),
        ProfilePreferences::new(ReconnectPreference::BoundedAutomatic, true, 250).unwrap(),
    )
    .unwrap()
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
    for (index, engine) in [Engine::PostgreSql, Engine::ClickHouse, Engine::Redis]
        .into_iter()
        .enumerate()
    {
        let profile = saved_profile(engine, index as u64 + 1);
        actor
            .create_profile(profile.persistable().unwrap())
            .unwrap();
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
    assert_eq!(reopened.health().unwrap().schema_version, 3);
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
