use tablerock_core::*;
use tablerock_persistence::PersistenceActor;

#[test]
fn create_sqlite_profile() {
    let path = std::env::temp_dir().join(format!("sqlite-prof-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let actor = PersistenceActor::open(&path).unwrap();
    let id = ProfileId::from_parts(IdParts::new(1, 1).unwrap()).unwrap();
    let props = ProfilePropertySet::new(vec![
        ProfilePropertyBinding::literal(
            ProfileProperty::Host,
            BoundedText::copy_from_str("local", ByteLimit::new(16)).unwrap(),
        )
        .unwrap(),
        ProfilePropertyBinding::literal(
            ProfileProperty::Port,
            BoundedText::copy_from_str("1", ByteLimit::new(5)).unwrap(),
        )
        .unwrap(),
        ProfilePropertyBinding::literal(
            ProfileProperty::DefaultContext,
            BoundedText::copy_from_str("samples/tablerock-sample.db", ByteLimit::new(128)).unwrap(),
        )
        .unwrap(),
    ])
    .unwrap();
    let connection = ProfileConnectionSnapshot::new(
        ProfileIdentity::new(
            id,
            Revision::INITIAL,
            Engine::Sqlite,
            ProfileName::new(BoundedText::copy_from_str("Sample Database", ByteLimit::new(64)).unwrap())
                .unwrap(),
        ),
        props,
        ProfilePolicy::new(
            TlsPolicy::Disabled,
            ProfileSafetyMode::ReadOnly,
            ProfileLimits::new(10_000, 30_000, 5_000, 16 * 1024 * 1024).unwrap(),
        ),
    )
    .unwrap();
    let agg = ProfileAggregate::new(
        connection,
        ProfileDurability::Saved,
        ProfileOrganization::new(None, vec![], false, 0, None).unwrap(),
        ProfilePreferences::new(ReconnectPreference::Manual, true, 250).unwrap(),
    )
    .unwrap();
    actor.create_profile(agg.persistable().unwrap()).unwrap();
    let loaded = actor.get_profile(id).unwrap().unwrap();
    assert_eq!(loaded.connection().engine(), Engine::Sqlite);
    actor.shutdown().unwrap();
    std::fs::remove_file(path).ok();
}
