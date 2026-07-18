use tablerock_core::{
    BoundedText, ByteLimit, OnePasswordObjectId, OnePasswordReference, OnePasswordSegment,
    ProfileProperty, ProfilePropertyBinding, ProfilePropertyError, ProfilePropertySet,
    PropertyValueSource, SecretSource, SecretSourceKind,
};

fn text(value: &str) -> BoundedText {
    BoundedText::copy_from_str(value, ByteLimit::new(70_000)).unwrap()
}

fn one_password_source() -> SecretSource {
    SecretSource::new(SecretSourceKind::OnePassword(
        OnePasswordReference::new(
            OnePasswordObjectId::parse("aaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap(),
            OnePasswordObjectId::parse("bbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap(),
            OnePasswordObjectId::parse("cccccccccccccccccccccccccc").unwrap(),
            None,
            OnePasswordSegment::parse("password").unwrap(),
            text("Production / Database / password"),
        )
        .unwrap(),
    ))
}

#[test]
fn ordinary_literals_are_structurally_rejected_for_secret_material() {
    for property in [
        ProfileProperty::Password,
        ProfileProperty::TlsClientPrivateKey,
        ProfileProperty::TlsClientPrivateKeyPassword,
        ProfileProperty::SshPassword,
        ProfileProperty::SshPrivateKey,
    ] {
        assert!(matches!(
            ProfilePropertyBinding::literal(property, text("must-not-survive")),
            Err(ProfilePropertyError::LiteralForbidden { property: actual }) if actual == property
        ));
    }

    let host = ProfilePropertyBinding::literal(ProfileProperty::Host, text("db.internal")).unwrap();
    assert_eq!(host.literal_value(), Some("db.internal"));
    assert_eq!(host.secret_source(), None);
    assert!(!format!("{host:?}").contains("db.internal"));
}

#[test]
fn references_can_supply_every_property_without_resolving_it() {
    for property in ProfileProperty::ALL {
        let binding = ProfilePropertyBinding::secret(property, one_password_source());
        assert_eq!(binding.property(), property);
        assert!(binding.literal_value().is_none());
        assert!(binding.secret_source().is_some());
        let debug = format!("{binding:?}");
        for forbidden in ["aaaaaaaa", "bbbbbbbb", "cccccccc", "password", "Production"] {
            assert!(!debug.contains(forbidden));
        }
    }
}

#[test]
fn literal_bounds_follow_property_semantics_before_allocation_crossings() {
    for (property, value) in [
        (ProfileProperty::Host, "".to_owned()),
        (ProfileProperty::Host, "h".repeat(254)),
        (ProfileProperty::Port, "123456".to_owned()),
        (ProfileProperty::DefaultContext, "d".repeat(129)),
        (ProfileProperty::Username, "u".repeat(129)),
        (ProfileProperty::TlsServerName, "s".repeat(254)),
        (ProfileProperty::TlsCaCertificate, "c".repeat(65_537)),
        (ProfileProperty::TlsClientCertificate, "c".repeat(65_537)),
    ] {
        assert!(ProfilePropertyBinding::literal(property, text(&value)).is_err());
    }

    for valid in ["1", "5432", "65535"] {
        assert!(ProfilePropertyBinding::literal(ProfileProperty::Port, text(valid)).is_ok());
        assert!(ProfilePropertyBinding::literal(ProfileProperty::SshPort, text(valid)).is_ok());
    }
    for invalid in ["+1", "54x2", "00000"] {
        assert!(matches!(
            ProfilePropertyBinding::literal(ProfileProperty::Port, text(invalid)),
            Err(ProfilePropertyError::InvalidPort)
        ));
        assert!(matches!(
            ProfilePropertyBinding::literal(ProfileProperty::SshPort, text(invalid)),
            Err(ProfilePropertyError::InvalidPort)
        ));
    }
}

#[test]
fn ssh_tunnel_properties_bind_as_optional_literals_and_secrets() {
    let set = ProfilePropertySet::new(vec![
        ProfilePropertyBinding::literal(ProfileProperty::Host, text("db.internal")).unwrap(),
        ProfilePropertyBinding::literal(ProfileProperty::Port, text("5432")).unwrap(),
        ProfilePropertyBinding::literal(ProfileProperty::SshHost, text("bastion.internal")).unwrap(),
        ProfilePropertyBinding::literal(ProfileProperty::SshPort, text("22")).unwrap(),
        ProfilePropertyBinding::literal(ProfileProperty::SshUsername, text("tunnel")).unwrap(),
        ProfilePropertyBinding::literal(
            ProfileProperty::SshKnownHostsPath,
            text("/var/lib/tablerock/known_hosts"),
        )
        .unwrap(),
        ProfilePropertyBinding::secret(ProfileProperty::SshPassword, one_password_source()),
    ])
    .unwrap();
    assert_eq!(
        set.literal(ProfileProperty::SshHost),
        Some("bastion.internal")
    );
    assert_eq!(set.literal(ProfileProperty::SshPort), Some("22"));
    assert!(set.binding(ProfileProperty::SshPassword).is_some());
    assert_eq!(ProfileProperty::ALL.len(), 16);
}

#[test]
fn property_sets_are_versioned_bounded_and_duplicate_free() {
    let host = ProfilePropertyBinding::literal(ProfileProperty::Host, text("localhost")).unwrap();
    let port = ProfilePropertyBinding::literal(ProfileProperty::Port, text("5432")).unwrap();
    let set = ProfilePropertySet::new(vec![host, port]).unwrap();
    assert_eq!(set.schema_version(), ProfilePropertySet::SCHEMA_VERSION);
    assert_eq!(set.len(), 2);
    assert_eq!(set.literal(ProfileProperty::Host), Some("localhost"));
    assert!(!format!("{set:?}").contains("localhost"));

    let duplicate = vec![
        ProfilePropertyBinding::literal(ProfileProperty::Host, text("one")).unwrap(),
        ProfilePropertyBinding::literal(ProfileProperty::Host, text("two")).unwrap(),
    ];
    assert_eq!(
        ProfilePropertySet::new(duplicate),
        Err(ProfilePropertyError::DuplicateProperty {
            property: ProfileProperty::Host,
        })
    );
    assert!(matches!(
        ProfilePropertySet::from_wire(2, Vec::new()),
        Err(ProfilePropertyError::UnsupportedSchemaVersion { .. })
    ));
}

#[test]
fn source_kind_is_explicit_without_exposing_values() {
    let literal =
        ProfilePropertyBinding::literal(ProfileProperty::Username, text("operator")).unwrap();
    assert_eq!(literal.source(), PropertyValueSource::Literal);
    assert_eq!(literal.literal_value(), Some("operator"));

    let prompt = ProfilePropertyBinding::secret(
        ProfileProperty::Password,
        SecretSource::new(SecretSourceKind::PromptOnConnect),
    );
    assert_eq!(prompt.source(), PropertyValueSource::SecretSource);
    assert_eq!(prompt.literal_value(), None);
}
