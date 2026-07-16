use std::{error::Error, fmt};

use crate::{BoundedText, Engine, ProfileId, Revision, SecretSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProfileProperty {
    Host,
    Port,
    DefaultContext,
    Username,
    Password,
    TlsServerName,
    TlsCaCertificate,
    TlsClientCertificate,
    TlsClientPrivateKey,
    TlsClientPrivateKeyPassword,
}

impl ProfileProperty {
    pub const ALL: [Self; 10] = [
        Self::Host,
        Self::Port,
        Self::DefaultContext,
        Self::Username,
        Self::Password,
        Self::TlsServerName,
        Self::TlsCaCertificate,
        Self::TlsClientCertificate,
        Self::TlsClientPrivateKey,
        Self::TlsClientPrivateKeyPassword,
    ];

    #[must_use]
    pub const fn permits_literal(self) -> bool {
        !matches!(
            self,
            Self::Password | Self::TlsClientPrivateKey | Self::TlsClientPrivateKeyPassword
        )
    }

    #[must_use]
    pub const fn literal_byte_limit(self) -> u64 {
        match self {
            Self::Host | Self::TlsServerName => 253,
            Self::Port => 5,
            Self::DefaultContext | Self::Username => 128,
            Self::TlsCaCertificate | Self::TlsClientCertificate => 65_536,
            Self::Password | Self::TlsClientPrivateKey | Self::TlsClientPrivateKeyPassword => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertyValueSource {
    Literal,
    SecretSource,
}

#[derive(PartialEq, Eq, Hash)]
enum ProfilePropertyValue {
    Literal(BoundedText),
    Secret(SecretSource),
}

#[derive(PartialEq, Eq, Hash)]
pub struct ProfilePropertyBinding {
    property: ProfileProperty,
    value: ProfilePropertyValue,
}

impl ProfilePropertyBinding {
    pub fn literal(
        property: ProfileProperty,
        value: BoundedText,
    ) -> Result<Self, ProfilePropertyError> {
        if !property.permits_literal() {
            return Err(ProfilePropertyError::LiteralForbidden { property });
        }
        let actual = value.len() as u64;
        let maximum = property.literal_byte_limit();
        if actual == 0 || actual > maximum {
            return Err(ProfilePropertyError::InvalidLiteralLength {
                property,
                actual,
                maximum,
            });
        }
        if property == ProfileProperty::Port {
            if !value.as_str().bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(ProfilePropertyError::InvalidPort);
            }
            let port = value
                .as_str()
                .parse::<u16>()
                .map_err(|_| ProfilePropertyError::InvalidPort)?;
            if port == 0 {
                return Err(ProfilePropertyError::InvalidPort);
            }
        }
        Ok(Self {
            property,
            value: ProfilePropertyValue::Literal(value),
        })
    }

    #[must_use]
    pub const fn secret(property: ProfileProperty, source: SecretSource) -> Self {
        Self {
            property,
            value: ProfilePropertyValue::Secret(source),
        }
    }

    #[must_use]
    pub const fn property(&self) -> ProfileProperty {
        self.property
    }

    #[must_use]
    pub const fn source(&self) -> PropertyValueSource {
        match self.value {
            ProfilePropertyValue::Literal(_) => PropertyValueSource::Literal,
            ProfilePropertyValue::Secret(_) => PropertyValueSource::SecretSource,
        }
    }

    #[must_use]
    pub fn literal_value(&self) -> Option<&str> {
        match &self.value {
            ProfilePropertyValue::Literal(value) => Some(value.as_str()),
            ProfilePropertyValue::Secret(_) => None,
        }
    }

    #[must_use]
    pub const fn secret_source(&self) -> Option<&SecretSource> {
        match &self.value {
            ProfilePropertyValue::Literal(_) => None,
            ProfilePropertyValue::Secret(source) => Some(source),
        }
    }
}

impl fmt::Debug for ProfilePropertyBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProfilePropertyBinding")
            .field("property", &self.property)
            .field("source", &self.source())
            .finish()
    }
}

#[derive(PartialEq, Eq)]
pub struct ProfilePropertySet {
    schema_version: u16,
    bindings: Vec<ProfilePropertyBinding>,
}

impl ProfilePropertySet {
    pub const SCHEMA_VERSION: u16 = 1;
    pub const MAX_BINDINGS: usize = ProfileProperty::ALL.len();

    pub fn new(bindings: Vec<ProfilePropertyBinding>) -> Result<Self, ProfilePropertyError> {
        Self::from_wire(Self::SCHEMA_VERSION, bindings)
    }

    pub fn from_wire(
        schema_version: u16,
        bindings: Vec<ProfilePropertyBinding>,
    ) -> Result<Self, ProfilePropertyError> {
        if schema_version != Self::SCHEMA_VERSION {
            return Err(ProfilePropertyError::UnsupportedSchemaVersion {
                actual: schema_version,
                supported: Self::SCHEMA_VERSION,
            });
        }
        if bindings.len() > Self::MAX_BINDINGS {
            return Err(ProfilePropertyError::TooManyBindings {
                actual: bindings.len(),
                maximum: Self::MAX_BINDINGS,
            });
        }
        for (index, binding) in bindings.iter().enumerate() {
            if bindings[..index]
                .iter()
                .any(|candidate| candidate.property == binding.property)
            {
                return Err(ProfilePropertyError::DuplicateProperty {
                    property: binding.property,
                });
            }
        }
        Ok(Self {
            schema_version,
            bindings,
        })
    }

    #[must_use]
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.bindings.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    #[must_use]
    pub fn binding(&self, property: ProfileProperty) -> Option<&ProfilePropertyBinding> {
        self.bindings
            .iter()
            .find(|binding| binding.property == property)
    }

    #[must_use]
    pub fn literal(&self, property: ProfileProperty) -> Option<&str> {
        self.binding(property)
            .and_then(ProfilePropertyBinding::literal_value)
    }
}

impl fmt::Debug for ProfilePropertySet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProfilePropertySet")
            .field("schema_version", &self.schema_version)
            .field("binding_count", &self.bindings.len())
            .field(
                "properties",
                &self
                    .bindings
                    .iter()
                    .map(ProfilePropertyBinding::property)
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfilePropertyError {
    LiteralForbidden {
        property: ProfileProperty,
    },
    InvalidLiteralLength {
        property: ProfileProperty,
        actual: u64,
        maximum: u64,
    },
    InvalidPort,
    TooManyBindings {
        actual: usize,
        maximum: usize,
    },
    DuplicateProperty {
        property: ProfileProperty,
    },
    UnsupportedSchemaVersion {
        actual: u16,
        supported: u16,
    },
}

impl fmt::Display for ProfilePropertyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LiteralForbidden { property } => {
                write!(formatter, "literal source is forbidden for {property:?}")
            }
            Self::InvalidLiteralLength {
                property,
                actual,
                maximum,
            } => write!(
                formatter,
                "{property:?} literal length {actual} is outside 1..={maximum} bytes"
            ),
            Self::InvalidPort => formatter.write_str("literal port must be in 1..=65535"),
            Self::TooManyBindings { actual, maximum } => {
                write!(formatter, "property count {actual} exceeds {maximum}")
            }
            Self::DuplicateProperty { property } => {
                write!(formatter, "duplicate profile property {property:?}")
            }
            Self::UnsupportedSchemaVersion { actual, supported } => write!(
                formatter,
                "profile-property schema version {actual} is unsupported; expected {supported}"
            ),
        }
    }
}

impl Error for ProfilePropertyError {}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ProfileName(BoundedText);

impl ProfileName {
    pub const MAX_BYTES: u64 = 128;

    pub fn new(value: BoundedText) -> Result<Self, ProfileBuildError> {
        let actual = value.len() as u64;
        if actual == 0 || actual > Self::MAX_BYTES {
            return Err(ProfileBuildError::InvalidNameLength {
                actual,
                maximum: Self::MAX_BYTES,
            });
        }
        if value.as_str().trim().is_empty() || value.as_str().chars().any(char::is_control) {
            return Err(ProfileBuildError::InvalidNameCharacter);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Debug for ProfileName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProfileName")
            .field("byte_len", &self.0.len())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProfileSafetyMode {
    ReadOnly,
    ConfirmWrites,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DangerousTlsAcknowledgement {
    LocalTestingOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TlsPolicy {
    Disabled,
    VerifySystemRoots,
    VerifyCustomCa,
    DangerousAcceptInvalidCertificate(DangerousTlsAcknowledgement),
}

impl TlsPolicy {
    #[must_use]
    pub const fn dangerous_accept_invalid_certificate(
        acknowledgement: DangerousTlsAcknowledgement,
    ) -> Self {
        Self::DangerousAcceptInvalidCertificate(acknowledgement)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProfileLimitField {
    ConnectTimeout,
    OperationTimeout,
    ResultRows,
    ResultBytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProfileLimits {
    connect_timeout_ms: u64,
    operation_timeout_ms: u64,
    max_result_rows: u64,
    max_result_bytes: u64,
}

impl ProfileLimits {
    pub const MAX_CONNECT_TIMEOUT_MS: u64 = 120_000;
    pub const MAX_OPERATION_TIMEOUT_MS: u64 = 3_600_000;
    pub const MAX_RESULT_ROWS: u64 = 1_000_000;
    pub const MAX_RESULT_BYTES: u64 = 1_073_741_824;

    pub fn new(
        connect_timeout_ms: u64,
        operation_timeout_ms: u64,
        max_result_rows: u64,
        max_result_bytes: u64,
    ) -> Result<Self, ProfileBuildError> {
        validate_limit(
            connect_timeout_ms,
            Self::MAX_CONNECT_TIMEOUT_MS,
            ProfileLimitField::ConnectTimeout,
        )?;
        validate_limit(
            operation_timeout_ms,
            Self::MAX_OPERATION_TIMEOUT_MS,
            ProfileLimitField::OperationTimeout,
        )?;
        validate_limit(
            max_result_rows,
            Self::MAX_RESULT_ROWS,
            ProfileLimitField::ResultRows,
        )?;
        validate_limit(
            max_result_bytes,
            Self::MAX_RESULT_BYTES,
            ProfileLimitField::ResultBytes,
        )?;
        Ok(Self {
            connect_timeout_ms,
            operation_timeout_ms,
            max_result_rows,
            max_result_bytes,
        })
    }

    #[must_use]
    pub const fn connect_timeout_ms(self) -> u64 {
        self.connect_timeout_ms
    }
    #[must_use]
    pub const fn operation_timeout_ms(self) -> u64 {
        self.operation_timeout_ms
    }
    #[must_use]
    pub const fn max_result_rows(self) -> u64 {
        self.max_result_rows
    }
    #[must_use]
    pub const fn max_result_bytes(self) -> u64 {
        self.max_result_bytes
    }
}

const fn validate_limit(
    actual: u64,
    maximum: u64,
    field: ProfileLimitField,
) -> Result<(), ProfileBuildError> {
    if actual == 0 || actual > maximum {
        Err(ProfileBuildError::InvalidLimit {
            field,
            actual,
            maximum,
        })
    } else {
        Ok(())
    }
}

#[derive(PartialEq, Eq)]
pub struct ProfileIdentity {
    id: ProfileId,
    revision: Revision,
    engine: Engine,
    name: ProfileName,
}

impl ProfileIdentity {
    #[must_use]
    pub const fn new(id: ProfileId, revision: Revision, engine: Engine, name: ProfileName) -> Self {
        Self {
            id,
            revision,
            engine,
            name,
        }
    }

    #[must_use]
    pub const fn id(&self) -> ProfileId {
        self.id
    }
    #[must_use]
    pub const fn revision(&self) -> Revision {
        self.revision
    }
    #[must_use]
    pub const fn engine(&self) -> Engine {
        self.engine
    }
    #[must_use]
    pub const fn name(&self) -> &ProfileName {
        &self.name
    }
}

impl fmt::Debug for ProfileIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProfileIdentity")
            .field("id", &self.id)
            .field("revision", &self.revision)
            .field("engine", &self.engine)
            .field("name", &self.name)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProfilePolicy {
    tls_policy: TlsPolicy,
    safety_mode: ProfileSafetyMode,
    limits: ProfileLimits,
}

impl ProfilePolicy {
    #[must_use]
    pub const fn new(
        tls_policy: TlsPolicy,
        safety_mode: ProfileSafetyMode,
        limits: ProfileLimits,
    ) -> Self {
        Self {
            tls_policy,
            safety_mode,
            limits,
        }
    }

    #[must_use]
    pub const fn tls_policy(self) -> TlsPolicy {
        self.tls_policy
    }
    #[must_use]
    pub const fn safety_mode(self) -> ProfileSafetyMode {
        self.safety_mode
    }
    #[must_use]
    pub const fn limits(self) -> ProfileLimits {
        self.limits
    }
}

#[derive(PartialEq, Eq)]
pub struct ProfileConnectionSnapshot {
    schema_version: u16,
    identity: ProfileIdentity,
    properties: ProfilePropertySet,
    policy: ProfilePolicy,
}

impl ProfileConnectionSnapshot {
    pub const SCHEMA_VERSION: u16 = 1;

    pub fn new(
        identity: ProfileIdentity,
        properties: ProfilePropertySet,
        policy: ProfilePolicy,
    ) -> Result<Self, ProfileBuildError> {
        Self::from_wire(Self::SCHEMA_VERSION, identity, properties, policy)
    }

    pub fn from_wire(
        schema_version: u16,
        identity: ProfileIdentity,
        properties: ProfilePropertySet,
        policy: ProfilePolicy,
    ) -> Result<Self, ProfileBuildError> {
        if schema_version != Self::SCHEMA_VERSION {
            return Err(ProfileBuildError::UnsupportedSchemaVersion {
                actual: schema_version,
                supported: Self::SCHEMA_VERSION,
            });
        }
        for property in [ProfileProperty::Host, ProfileProperty::Port] {
            if properties.binding(property).is_none() {
                return Err(ProfileBuildError::MissingRequiredProperty { property });
            }
        }
        validate_tls(&properties, policy.tls_policy())?;
        Ok(Self {
            schema_version,
            identity,
            properties,
            policy,
        })
    }

    #[must_use]
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }
    #[must_use]
    pub const fn id(&self) -> ProfileId {
        self.identity.id()
    }
    #[must_use]
    pub const fn revision(&self) -> Revision {
        self.identity.revision()
    }
    #[must_use]
    pub const fn engine(&self) -> Engine {
        self.identity.engine()
    }
    #[must_use]
    pub const fn name(&self) -> &ProfileName {
        self.identity.name()
    }
    #[must_use]
    pub const fn properties(&self) -> &ProfilePropertySet {
        &self.properties
    }
    #[must_use]
    pub const fn tls_policy(&self) -> TlsPolicy {
        self.policy.tls_policy()
    }
    #[must_use]
    pub const fn safety_mode(&self) -> ProfileSafetyMode {
        self.policy.safety_mode()
    }
    #[must_use]
    pub const fn limits(&self) -> ProfileLimits {
        self.policy.limits()
    }
}

impl fmt::Debug for ProfileConnectionSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProfileConnectionSnapshot")
            .field("schema_version", &self.schema_version)
            .field("identity", &self.identity)
            .field("properties", &self.properties)
            .field("policy", &self.policy)
            .finish()
    }
}

fn validate_tls(
    properties: &ProfilePropertySet,
    policy: TlsPolicy,
) -> Result<(), ProfileBuildError> {
    const TLS_PROPERTIES: [ProfileProperty; 5] = [
        ProfileProperty::TlsServerName,
        ProfileProperty::TlsCaCertificate,
        ProfileProperty::TlsClientCertificate,
        ProfileProperty::TlsClientPrivateKey,
        ProfileProperty::TlsClientPrivateKeyPassword,
    ];
    if policy == TlsPolicy::Disabled {
        for property in TLS_PROPERTIES {
            if properties.binding(property).is_some() {
                return Err(ProfileBuildError::TlsPropertyForbidden { property });
            }
        }
        return Ok(());
    }
    if policy == TlsPolicy::VerifyCustomCa
        && properties
            .binding(ProfileProperty::TlsCaCertificate)
            .is_none()
    {
        return Err(ProfileBuildError::MissingTlsProperty {
            property: ProfileProperty::TlsCaCertificate,
        });
    }
    if matches!(
        policy,
        TlsPolicy::VerifySystemRoots | TlsPolicy::DangerousAcceptInvalidCertificate(_)
    ) && properties
        .binding(ProfileProperty::TlsCaCertificate)
        .is_some()
    {
        return Err(ProfileBuildError::TlsPropertyForbidden {
            property: ProfileProperty::TlsCaCertificate,
        });
    }

    let certificate = properties
        .binding(ProfileProperty::TlsClientCertificate)
        .is_some();
    let private_key = properties
        .binding(ProfileProperty::TlsClientPrivateKey)
        .is_some();
    if certificate && !private_key {
        return Err(ProfileBuildError::MissingTlsProperty {
            property: ProfileProperty::TlsClientPrivateKey,
        });
    }
    if private_key && !certificate {
        return Err(ProfileBuildError::MissingTlsProperty {
            property: ProfileProperty::TlsClientCertificate,
        });
    }
    if properties
        .binding(ProfileProperty::TlsClientPrivateKeyPassword)
        .is_some()
        && !private_key
    {
        return Err(ProfileBuildError::MissingTlsProperty {
            property: ProfileProperty::TlsClientPrivateKey,
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileBuildError {
    InvalidNameLength {
        actual: u64,
        maximum: u64,
    },
    InvalidNameCharacter,
    InvalidLimit {
        field: ProfileLimitField,
        actual: u64,
        maximum: u64,
    },
    MissingRequiredProperty {
        property: ProfileProperty,
    },
    MissingTlsProperty {
        property: ProfileProperty,
    },
    TlsPropertyForbidden {
        property: ProfileProperty,
    },
    UnsupportedSchemaVersion {
        actual: u16,
        supported: u16,
    },
}

impl fmt::Display for ProfileBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidNameLength { .. } => "profile name length is invalid",
            Self::InvalidNameCharacter => "profile name contains invalid characters",
            Self::InvalidLimit { .. } => "profile limit is outside its finite owner bound",
            Self::MissingRequiredProperty { .. } => "profile is missing a required property",
            Self::MissingTlsProperty { .. } => "TLS configuration is incomplete",
            Self::TlsPropertyForbidden { .. } => "TLS property contradicts selected policy",
            Self::UnsupportedSchemaVersion { .. } => {
                "profile connection schema version is unsupported"
            }
        })
    }
}

impl Error for ProfileBuildError {}
