use std::{error::Error, fmt};

use crate::{BoundedText, SecretSource};

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
