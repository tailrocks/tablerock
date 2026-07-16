use tablerock_core::{
    BoundedText, ByteLimit, DangerousTlsAcknowledgement, Engine, IdParts, ProfileBuildError,
    ProfileConnectionSnapshot, ProfileId, ProfileIdentity, ProfileLimits, ProfileName,
    ProfilePolicy, ProfileProperty, ProfilePropertyBinding, ProfilePropertySet, ProfileSafetyMode,
    Revision, SecretSource, SecretSourceKind, TlsPolicy,
};

fn text(value: &str) -> BoundedText {
    BoundedText::copy_from_str(value, ByteLimit::new(70_000)).unwrap()
}

fn profile_id() -> ProfileId {
    ProfileId::from_parts(IdParts::new(1, 2).unwrap()).unwrap()
}

fn binding(property: ProfileProperty, value: &str) -> ProfilePropertyBinding {
    ProfilePropertyBinding::literal(property, text(value)).unwrap()
}

fn required_properties() -> ProfilePropertySet {
    ProfilePropertySet::new(vec![
        binding(ProfileProperty::Host, "database.internal"),
        binding(ProfileProperty::Port, "5432"),
    ])
    .unwrap()
}

fn limits() -> ProfileLimits {
    ProfileLimits::new(10_000, 30_000, 10_000, 64 * 1024 * 1024).unwrap()
}

fn identity(engine: Engine, name: &str) -> ProfileIdentity {
    ProfileIdentity::new(
        profile_id(),
        Revision::INITIAL,
        engine,
        ProfileName::new(text(name)).unwrap(),
    )
}

fn policy(tls: TlsPolicy, safety: ProfileSafetyMode) -> ProfilePolicy {
    ProfilePolicy::new(tls, safety, limits())
}

#[test]
fn complete_snapshot_is_versioned_immutable_and_redacted_for_every_engine() {
    for engine in [Engine::PostgreSql, Engine::ClickHouse, Engine::Redis] {
        let snapshot = ProfileConnectionSnapshot::new(
            identity(engine, "Production database"),
            required_properties(),
            policy(TlsPolicy::VerifySystemRoots, ProfileSafetyMode::ReadOnly),
        )
        .unwrap();
        assert_eq!(
            snapshot.schema_version(),
            ProfileConnectionSnapshot::SCHEMA_VERSION
        );
        assert_eq!(snapshot.engine(), engine);
        assert_eq!(snapshot.id(), profile_id());
        assert_eq!(snapshot.revision(), Revision::INITIAL);
        assert_eq!(snapshot.name().as_str(), "Production database");
        assert_eq!(
            snapshot.properties().literal(ProfileProperty::Port),
            Some("5432")
        );
        let debug = format!("{snapshot:?}");
        assert!(!debug.contains("Production database"));
        assert!(!debug.contains("database.internal"));
        assert!(!debug.contains("5432"));
    }
}

#[test]
fn every_engine_requires_bounded_host_and_port_sources() {
    for engine in [Engine::PostgreSql, Engine::ClickHouse, Engine::Redis] {
        for missing in [ProfileProperty::Host, ProfileProperty::Port] {
            let present = if missing == ProfileProperty::Host {
                binding(ProfileProperty::Port, "5432")
            } else {
                binding(ProfileProperty::Host, "localhost")
            };
            assert!(matches!(
                ProfileConnectionSnapshot::new(
                    identity(engine, "Incomplete"),
                    ProfilePropertySet::new(vec![present]).unwrap(),
                    policy(TlsPolicy::Disabled, ProfileSafetyMode::ConfirmWrites),
                ),
                Err(ProfileBuildError::MissingRequiredProperty { property }) if property == missing
            ));
        }
    }

    let secret_host = ProfilePropertyBinding::secret(
        ProfileProperty::Host,
        SecretSource::new(SecretSourceKind::PromptOnConnect),
    );
    let properties =
        ProfilePropertySet::new(vec![secret_host, binding(ProfileProperty::Port, "6379")]).unwrap();
    assert!(
        ProfileConnectionSnapshot::new(
            identity(Engine::Redis, "Prompt host"),
            properties,
            policy(TlsPolicy::Disabled, ProfileSafetyMode::ReadOnly),
        )
        .is_ok()
    );
}

#[test]
fn tls_policy_rejects_contradictory_or_incomplete_property_sets() {
    let with = |extra: Vec<ProfilePropertyBinding>| {
        let mut values = vec![
            binding(ProfileProperty::Host, "localhost"),
            binding(ProfileProperty::Port, "5432"),
        ];
        values.extend(extra);
        ProfilePropertySet::new(values).unwrap()
    };
    let build = |properties, tls| {
        ProfileConnectionSnapshot::new(
            identity(Engine::PostgreSql, "TLS fixture"),
            properties,
            policy(tls, ProfileSafetyMode::ReadOnly),
        )
    };

    assert!(matches!(
        build(
            with(vec![binding(ProfileProperty::TlsServerName, "db.local")]),
            TlsPolicy::Disabled,
        ),
        Err(ProfileBuildError::TlsPropertyForbidden { .. })
    ));
    assert!(matches!(
        build(with(Vec::new()), TlsPolicy::VerifyCustomCa),
        Err(ProfileBuildError::MissingTlsProperty {
            property: ProfileProperty::TlsCaCertificate,
        })
    ));
    assert!(matches!(
        build(
            with(vec![binding(ProfileProperty::TlsCaCertificate, "CA")]),
            TlsPolicy::VerifySystemRoots,
        ),
        Err(ProfileBuildError::TlsPropertyForbidden {
            property: ProfileProperty::TlsCaCertificate,
        })
    ));
    assert!(matches!(
        build(
            with(vec![binding(ProfileProperty::TlsClientCertificate, "CERT")]),
            TlsPolicy::VerifySystemRoots,
        ),
        Err(ProfileBuildError::MissingTlsProperty {
            property: ProfileProperty::TlsClientPrivateKey,
        })
    ));
    let private_key = ProfilePropertyBinding::secret(
        ProfileProperty::TlsClientPrivateKey,
        SecretSource::new(SecretSourceKind::PromptOnConnect),
    );
    assert!(
        build(
            with(vec![
                binding(ProfileProperty::TlsClientCertificate, "CERT"),
                private_key,
            ]),
            TlsPolicy::VerifySystemRoots,
        )
        .is_ok()
    );
    let orphan_password = ProfilePropertyBinding::secret(
        ProfileProperty::TlsClientPrivateKeyPassword,
        SecretSource::new(SecretSourceKind::PromptOnConnect),
    );
    assert!(matches!(
        build(with(vec![orphan_password]), TlsPolicy::VerifySystemRoots,),
        Err(ProfileBuildError::MissingTlsProperty {
            property: ProfileProperty::TlsClientPrivateKey,
        })
    ));
}

#[test]
fn dangerous_tls_and_profile_limits_require_explicit_bounded_values() {
    assert_eq!(
        TlsPolicy::dangerous_accept_invalid_certificate(
            DangerousTlsAcknowledgement::LocalTestingOnly
        ),
        TlsPolicy::DangerousAcceptInvalidCertificate(DangerousTlsAcknowledgement::LocalTestingOnly)
    );
    for invalid in [
        ProfileLimits::new(0, 30_000, 10_000, 64),
        ProfileLimits::new(10_000, 0, 10_000, 64),
        ProfileLimits::new(10_000, 30_000, 0, 64),
        ProfileLimits::new(10_000, 30_000, 10_000, 0),
        ProfileLimits::new(120_001, 30_000, 10_000, 64),
        ProfileLimits::new(10_000, 3_600_001, 10_000, 64),
        ProfileLimits::new(10_000, 30_000, 1_000_001, 64),
        ProfileLimits::new(10_000, 30_000, 10_000, 1_073_741_825),
    ] {
        assert!(invalid.is_err());
    }
    let limits = limits();
    assert_eq!(limits.connect_timeout_ms(), 10_000);
    assert_eq!(limits.operation_timeout_ms(), 30_000);
    assert_eq!(limits.max_result_rows(), 10_000);
    assert_eq!(limits.max_result_bytes(), 64 * 1024 * 1024);
    assert!(ProfileLimits::new(120_000, 3_600_000, 1_000_000, 1_073_741_824).is_ok());
    assert!(
        ProfileConnectionSnapshot::new(
            identity(Engine::ClickHouse, "Dangerous TLS fixture"),
            required_properties(),
            ProfilePolicy::new(
                TlsPolicy::dangerous_accept_invalid_certificate(
                    DangerousTlsAcknowledgement::LocalTestingOnly,
                ),
                ProfileSafetyMode::ReadOnly,
                limits,
            ),
        )
        .is_ok()
    );
}

#[test]
fn names_and_wire_versions_fail_closed() {
    for name in ["", "   ", "line\nbreak", &"n".repeat(129)] {
        assert!(ProfileName::new(text(name)).is_err());
    }
    assert!(matches!(
        ProfileConnectionSnapshot::from_wire(
            2,
            identity(Engine::Redis, "Unknown schema"),
            required_properties(),
            policy(TlsPolicy::Disabled, ProfileSafetyMode::ReadOnly),
        ),
        Err(ProfileBuildError::UnsupportedSchemaVersion { .. })
    ));
}
