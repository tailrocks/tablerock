use tablerock_core::{
    BoundedBytes, BoundedText, ByteLimit, DangerousPlaintext, EnvironmentReference,
    KeychainReference, OnePasswordObjectId, OnePasswordReference, OnePasswordSegment,
    PlaintextAcknowledgement, SecretBuildError, SecretPersistenceRisk, SecretSource,
    SecretSourceKind,
};

fn object_id(seed: u8) -> OnePasswordObjectId {
    let text = std::iter::repeat_n(char::from(seed), 26).collect::<String>();
    OnePasswordObjectId::parse(&text).unwrap()
}

fn segment(text: &str) -> OnePasswordSegment {
    OnePasswordSegment::parse(text).unwrap()
}

#[test]
fn one_password_reference_is_account_pinned_structured_and_redacted() {
    let account = object_id(b'a');
    let field = segment("password");
    assert!(!format!("{account:?}").contains("aaaaaaaa"));
    assert!(!format!("{field:?}").contains("password"));
    let reference = OnePasswordReference::new(
        account,
        object_id(b'b'),
        object_id(b'c'),
        None,
        field,
        BoundedText::copy_from_str("Production / Database / password", ByteLimit::new(32)).unwrap(),
    )
    .unwrap();
    let reference_debug = format!("{reference:?}");
    assert!(!reference_debug.contains("Production"));
    assert!(!reference_debug.contains("password"));
    let source = SecretSource::new(SecretSourceKind::OnePassword(reference));

    assert_eq!(source.schema_version(), SecretSource::SCHEMA_VERSION);
    assert_eq!(
        source.persistence_risk(),
        SecretPersistenceRisk::ReferenceOnly
    );
    let debug = format!("{source:?}");
    assert!(!debug.contains("Production"));
    assert!(!debug.contains("password"));
    assert!(!debug.contains("aaaaaaaa"));
}

#[test]
fn one_password_secret_reference_uri_and_compact_wire_round_trip() {
    let with_section = OnePasswordReference::new(
        object_id(b'a'),
        object_id(b'b'),
        object_id(b'c'),
        Some(segment("database")),
        segment("password"),
        BoundedText::copy_from_str("db/password", ByteLimit::new(32)).unwrap(),
    )
    .unwrap();
    assert_eq!(
        with_section.secret_reference_uri(),
        format!(
            "op://{}/{}/{}/{}",
            object_id(b'b').as_str(),
            object_id(b'c').as_str(),
            "database",
            "password"
        )
    );
    let wire = with_section.to_compact_wire();
    let parsed = OnePasswordReference::from_compact_wire(&wire).unwrap();
    assert_eq!(parsed.account_id().as_str(), object_id(b'a').as_str());
    assert_eq!(parsed.vault_id().as_str(), object_id(b'b').as_str());
    assert_eq!(parsed.item_id().as_str(), object_id(b'c').as_str());
    assert_eq!(parsed.section_id().unwrap().as_str(), "database");
    assert_eq!(parsed.field_id().as_str(), "password");
    assert_eq!(
        parsed.secret_reference_uri(),
        with_section.secret_reference_uri()
    );

    let no_section = OnePasswordReference::new(
        object_id(b'a'),
        object_id(b'b'),
        object_id(b'c'),
        None,
        segment("password"),
        BoundedText::copy_from_str("password", ByteLimit::new(32)).unwrap(),
    )
    .unwrap();
    assert_eq!(
        no_section.secret_reference_uri(),
        format!(
            "op://{}/{}/{}",
            object_id(b'b').as_str(),
            object_id(b'c').as_str(),
            "password"
        )
    );
    let parsed_plain =
        OnePasswordReference::from_compact_wire(&no_section.to_compact_wire()).unwrap();
    assert!(parsed_plain.section_id().is_none());
    assert!(matches!(
        OnePasswordReference::from_compact_wire("too few"),
        Err(SecretBuildError::InvalidOnePasswordCompact)
    ));
}

#[test]
fn reference_identifiers_and_environment_names_fail_closed() {
    assert!(matches!(
        OnePasswordObjectId::parse("too-short"),
        Err(SecretBuildError::InvalidObjectIdLength { .. })
    ));
    assert!(matches!(
        OnePasswordObjectId::parse("aaaaaaaaaaaaaaaaaaaaaaaaa/"),
        Err(SecretBuildError::InvalidReferenceCharacter { .. })
    ));
    assert!(matches!(
        OnePasswordSegment::parse("field/name"),
        Err(SecretBuildError::InvalidReferenceCharacter { .. })
    ));
    assert_eq!(
        EnvironmentReference::parse("9PASSWORD"),
        Err(SecretBuildError::InvalidEnvironmentName)
    );
    assert_eq!(
        EnvironmentReference::parse("DB_PASSWORD").unwrap().as_str(),
        "DB_PASSWORD"
    );

    for invalid_segment in ["", &"x".repeat(129)] {
        assert!(OnePasswordSegment::parse(invalid_segment).is_err());
    }
    assert!(EnvironmentReference::parse(&format!("A{}", "X".repeat(128))).is_err());

    for breadcrumb in ["", &"x".repeat(257)] {
        let value = BoundedText::copy_from_str(breadcrumb, ByteLimit::new(257)).unwrap();
        assert!(
            OnePasswordReference::new(
                object_id(b'a'),
                object_id(b'b'),
                object_id(b'c'),
                None,
                segment("password"),
                value,
            )
            .is_err()
        );
    }

    for bytes in [Vec::new(), vec![0; 4097]] {
        assert!(
            KeychainReference::new(BoundedBytes::from_vec(bytes, ByteLimit::new(4097)).unwrap(),)
                .is_err()
        );
    }
}

#[test]
fn dangerous_plaintext_requires_typed_acknowledgement_and_stays_redacted() {
    let mut plaintext = DangerousPlaintext::new(
        b"local-test-secret".to_vec(),
        PlaintextAcknowledgement::LocalTestingOnly,
    )
    .unwrap();
    assert!(!format!("{plaintext:?}").contains("local-test-secret"));
    plaintext.clear();
    assert!(plaintext.bytes().is_empty());
    let source = SecretSource::new(SecretSourceKind::DangerousPlaintext(plaintext));
    assert_eq!(
        source.persistence_risk(),
        SecretPersistenceRisk::DangerousPlaintext
    );
    assert!(!format!("{source:?}").contains("local-test-secret"));

    for bytes in [Vec::new(), vec![0; 65_537]] {
        assert!(
            DangerousPlaintext::new(bytes, PlaintextAcknowledgement::LocalTestingOnly,).is_err()
        );
    }
}

#[test]
fn every_secret_source_has_explicit_resolution_and_version_behavior() {
    let sources = [
        SecretSource::new(SecretSourceKind::PromptOnConnect),
        SecretSource::new(SecretSourceKind::HostEnvironment(
            EnvironmentReference::parse("DB_PASSWORD").unwrap(),
        )),
        SecretSource::new(SecretSourceKind::Keychain(
            KeychainReference::new(
                BoundedBytes::copy_from_slice(b"persistent-ref", ByteLimit::new(64)).unwrap(),
            )
            .unwrap(),
        )),
    ];
    let environment = EnvironmentReference::parse("ANOTHER_PASSWORD").unwrap();
    assert!(!format!("{environment:?}").contains("ANOTHER_PASSWORD"));
    let keychain = KeychainReference::new(
        BoundedBytes::copy_from_slice(b"another-persistent-ref", ByteLimit::new(64)).unwrap(),
    )
    .unwrap();
    assert!(!format!("{keychain:?}").contains("another-persistent-ref"));
    assert_eq!(sources[0].persistence_risk(), SecretPersistenceRisk::Prompt);
    assert_eq!(
        sources[1].persistence_risk(),
        SecretPersistenceRisk::ReferenceOnly
    );
    assert!(sources[2].requires_native_adapter());
    assert!(!format!("{:?}", sources[1]).contains("DB_PASSWORD"));
    assert!(!format!("{:?}", sources[2]).contains("persistent-ref"));

    assert_eq!(
        SecretSource::from_wire(2, SecretSourceKind::PromptOnConnect),
        Err(SecretBuildError::UnsupportedSchemaVersion {
            actual: 2,
            supported: 1,
        })
    );
}
