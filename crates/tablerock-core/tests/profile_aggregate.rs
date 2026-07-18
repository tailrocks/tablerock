use tablerock_core::{
    BoundedText, ByteLimit, Engine, IdParts, ProfileAggregate, ProfileAggregateError,
    ProfileConnectionSnapshot, ProfileDurability, ProfileGroupName, ProfileId, ProfileIdentity,
    ProfileLimits, ProfileOrganization, ProfilePolicy, ProfilePreferences, ProfileProperty,
    ProfilePropertyBinding, ProfilePropertySet, ProfileSafetyMode, ProfileTag, ProfileUpdateError,
    ReconnectPreference, Revision, TlsPolicy,
};

fn text(value: &str) -> BoundedText {
    BoundedText::copy_from_str(value, ByteLimit::new(10_000)).unwrap()
}

fn profile_id(low: u64) -> ProfileId {
    ProfileId::from_parts(IdParts::new(1, low).unwrap()).unwrap()
}

fn connection(id: ProfileId, revision: Revision) -> ProfileConnectionSnapshot {
    let properties = ProfilePropertySet::new(vec![
        ProfilePropertyBinding::literal(ProfileProperty::Host, text("db.internal")).unwrap(),
        ProfilePropertyBinding::literal(ProfileProperty::Port, text("5432")).unwrap(),
    ])
    .unwrap();
    ProfileConnectionSnapshot::new(
        ProfileIdentity::new(
            id,
            revision,
            Engine::PostgreSql,
            tablerock_core::ProfileName::new(text("Production")).unwrap(),
        ),
        properties,
        ProfilePolicy::new(
            TlsPolicy::VerifySystemRoots,
            ProfileSafetyMode::ReadOnly,
            ProfileLimits::new(10_000, 30_000, 10_000, 64 * 1024 * 1024).unwrap(),
        ),
    )
    .unwrap()
}

fn preferences() -> ProfilePreferences {
    ProfilePreferences::new(ReconnectPreference::BoundedAutomatic, true, 500).unwrap()
}

fn organization() -> ProfileOrganization {
    ProfileOrganization::new(
        Some(ProfileGroupName::new(text("Operations")).unwrap()),
        vec![
            ProfileTag::new(text("production")).unwrap(),
            ProfileTag::new(text("critical")).unwrap(),
        ],
        true,
        7,
        Some(tablerock_core::EnvironmentTag::Production),
    )
    .unwrap()
}

#[test]
fn saved_profile_aggregate_is_versioned_bounded_and_redacted() {
    let aggregate = ProfileAggregate::new(
        connection(profile_id(1), Revision::INITIAL),
        ProfileDurability::Saved,
        organization(),
        preferences(),
    )
    .unwrap();
    assert_eq!(aggregate.schema_version(), ProfileAggregate::SCHEMA_VERSION);
    let persistable = aggregate.persistable().expect("saved profile token");
    assert_eq!(persistable.profile().connection().id(), profile_id(1));
    assert_eq!(aggregate.connection().id(), profile_id(1));
    assert_eq!(aggregate.organization().tags().len(), 2);
    assert_eq!(aggregate.preferences().preferred_page_rows(), 500);
    let debug = format!("{aggregate:?}");
    for forbidden in [
        "Production",
        "db.internal",
        "Operations",
        "production",
        "critical",
    ] {
        assert!(!debug.contains(forbidden));
    }
}

#[test]
fn temporary_profiles_are_memory_only_and_cannot_carry_saved_organization() {
    let temporary = ProfileAggregate::new(
        connection(profile_id(2), Revision::INITIAL),
        ProfileDurability::Temporary,
        ProfileOrganization::empty(),
        preferences(),
    )
    .unwrap();
    assert!(temporary.persistable().is_none());
    assert!(matches!(
        ProfileAggregate::new(
            connection(profile_id(3), Revision::INITIAL),
            ProfileDurability::Temporary,
            organization(),
            preferences(),
        ),
        Err(ProfileAggregateError::TemporaryOrganizationForbidden)
    ));
    assert!(matches!(
        ProfileAggregate::from_wire(
            2,
            connection(profile_id(3), Revision::INITIAL),
            ProfileDurability::Saved,
            ProfileOrganization::empty(),
            preferences(),
        ),
        Err(ProfileAggregateError::UnsupportedSchemaVersion { .. })
    ));
}

#[test]
fn organization_names_tags_and_cardinality_fail_closed() {
    for invalid in ["", "   ", "line\nbreak", &"g".repeat(129)] {
        assert!(ProfileGroupName::new(text(invalid)).is_err());
    }
    for invalid in ["", " ", "line\nbreak", &"t".repeat(65)] {
        assert!(ProfileTag::new(text(invalid)).is_err());
    }
    assert!(matches!(
        ProfileOrganization::new(
            None,
            vec![
                ProfileTag::new(text("same")).unwrap(),
                ProfileTag::new(text("same")).unwrap(),
            ],
            false,
            0,
            None,
        ),
        Err(ProfileAggregateError::DuplicateTag { .. })
    ));
    let too_many = (0..33)
        .map(|index| ProfileTag::new(text(&format!("tag-{index}"))).unwrap())
        .collect();
    assert!(matches!(
        ProfileOrganization::new(None, too_many, false, 0, None),
        Err(ProfileAggregateError::TooManyTags { .. })
    ));
}

#[test]
fn preferences_have_explicit_finite_page_bounds() {
    for invalid in [0, 501] {
        assert!(ProfilePreferences::new(ReconnectPreference::Manual, false, invalid).is_err());
    }
    for valid in [1, 500] {
        let value = ProfilePreferences::new(ReconnectPreference::Manual, false, valid).unwrap();
        assert_eq!(value.preferred_page_rows(), valid);
    }
}

#[test]
fn replacement_gate_rejects_stale_cross_identity_and_nonsequential_updates() {
    let current = ProfileAggregate::new(
        connection(profile_id(4), Revision::from_wire_u64(8)),
        ProfileDurability::Saved,
        organization(),
        preferences(),
    )
    .unwrap();
    let next = ProfileAggregate::new(
        connection(profile_id(4), Revision::from_wire_u64(9)),
        ProfileDurability::Saved,
        organization(),
        preferences(),
    )
    .unwrap();
    assert!(
        current
            .validate_replacement(Revision::from_wire_u64(8), &next)
            .is_ok()
    );
    assert!(matches!(
        current.validate_replacement(Revision::from_wire_u64(7), &next),
        Err(ProfileUpdateError::StaleRevision { .. })
    ));

    let wrong_id = ProfileAggregate::new(
        connection(profile_id(5), Revision::from_wire_u64(9)),
        ProfileDurability::Saved,
        ProfileOrganization::empty(),
        preferences(),
    )
    .unwrap();
    assert_eq!(
        current.validate_replacement(Revision::from_wire_u64(8), &wrong_id),
        Err(ProfileUpdateError::IdentityMismatch)
    );
    let skipped = ProfileAggregate::new(
        connection(profile_id(4), Revision::from_wire_u64(10)),
        ProfileDurability::Saved,
        ProfileOrganization::empty(),
        preferences(),
    )
    .unwrap();
    assert!(matches!(
        current.validate_replacement(Revision::from_wire_u64(8), &skipped),
        Err(ProfileUpdateError::NonSequentialRevision { .. })
    ));
    let durability_change = ProfileAggregate::new(
        connection(profile_id(4), Revision::from_wire_u64(9)),
        ProfileDurability::Temporary,
        ProfileOrganization::empty(),
        preferences(),
    )
    .unwrap();
    assert_eq!(
        current.validate_replacement(Revision::from_wire_u64(8), &durability_change),
        Err(ProfileUpdateError::DurabilityChange)
    );

    let exhausted = ProfileAggregate::new(
        connection(profile_id(6), Revision::from_wire_u64(u64::MAX)),
        ProfileDurability::Saved,
        ProfileOrganization::empty(),
        preferences(),
    )
    .unwrap();
    let impossible_next = ProfileAggregate::new(
        connection(profile_id(6), Revision::from_wire_u64(u64::MAX)),
        ProfileDurability::Saved,
        ProfileOrganization::empty(),
        preferences(),
    )
    .unwrap();
    assert_eq!(
        exhausted.validate_replacement(Revision::from_wire_u64(u64::MAX), &impossible_next),
        Err(ProfileUpdateError::RevisionExhausted)
    );
}

#[test]
fn environment_tag_wire_and_production_warning() {
    assert!(tablerock_core::EnvironmentTag::Production.is_production_warning());
    assert!(!tablerock_core::EnvironmentTag::Staging.is_production_warning());
    assert!(
        !tablerock_core::EnvironmentTag::Custom(ProfileTag::new(text("lab")).unwrap())
            .is_production_warning()
    );
    let custom = tablerock_core::EnvironmentTag::from_wire(5, Some(text("lab"))).unwrap();
    assert_eq!(custom.custom_label(), Some("lab"));
    assert_eq!(custom.wire_kind(), 5);
    assert!(tablerock_core::EnvironmentTag::from_wire(5, None).is_err());
    assert!(tablerock_core::EnvironmentTag::from_wire(9, None).is_err());
    let org = ProfileOrganization::new(
        None,
        Vec::new(),
        false,
        0,
        Some(tablerock_core::EnvironmentTag::Production),
    )
    .unwrap();
    assert!(!org.is_empty());
    assert!(org.environment().unwrap().is_production_warning());
}
