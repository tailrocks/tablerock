use std::{error::Error, fmt};

use zeroize::Zeroize;

use crate::{BoundedBytes, BoundedText, ByteLimit};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct OnePasswordObjectId(BoundedText);

impl OnePasswordObjectId {
    pub const BYTE_LEN: u64 = 26;

    pub fn parse(value: &str) -> Result<Self, SecretBuildError> {
        if value.len() as u64 != Self::BYTE_LEN {
            return Err(SecretBuildError::InvalidObjectIdLength {
                actual: value.len() as u64,
                expected: Self::BYTE_LEN,
            });
        }
        if let Some((byte_index, _)) = value
            .bytes()
            .enumerate()
            .find(|(_, byte)| !byte.is_ascii_alphanumeric())
        {
            return Err(SecretBuildError::InvalidReferenceCharacter {
                byte_index: byte_index as u64,
            });
        }
        Ok(Self(
            BoundedText::copy_from_str(value, ByteLimit::new(Self::BYTE_LEN))
                .expect("validated 1Password object ID length"),
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Debug for OnePasswordObjectId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OnePasswordObjectId(REDACTED)")
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct OnePasswordSegment(BoundedText);

impl OnePasswordSegment {
    pub const MAX_BYTES: u64 = 128;

    pub fn parse(value: &str) -> Result<Self, SecretBuildError> {
        check_nonempty_limit(
            value.len(),
            Self::MAX_BYTES,
            SecretField::OnePasswordSegment,
        )?;
        if let Some((byte_index, _)) = value
            .bytes()
            .enumerate()
            .find(|(_, byte)| !byte.is_ascii_alphanumeric() && !matches!(byte, b'_' | b'-' | b'.'))
        {
            return Err(SecretBuildError::InvalidReferenceCharacter {
                byte_index: byte_index as u64,
            });
        }
        Ok(Self(
            BoundedText::copy_from_str(value, ByteLimit::new(Self::MAX_BYTES))
                .expect("validated 1Password segment length"),
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Debug for OnePasswordSegment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OnePasswordSegment(REDACTED)")
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct OnePasswordReference {
    account_id: OnePasswordObjectId,
    vault_id: OnePasswordObjectId,
    item_id: OnePasswordObjectId,
    section_id: Option<OnePasswordSegment>,
    field_id: OnePasswordSegment,
    breadcrumb: BoundedText,
}

impl OnePasswordReference {
    pub const MAX_BREADCRUMB_BYTES: u64 = 256;

    pub fn new(
        account_id: OnePasswordObjectId,
        vault_id: OnePasswordObjectId,
        item_id: OnePasswordObjectId,
        section_id: Option<OnePasswordSegment>,
        field_id: OnePasswordSegment,
        breadcrumb: BoundedText,
    ) -> Result<Self, SecretBuildError> {
        check_nonempty_limit(
            breadcrumb.len(),
            Self::MAX_BREADCRUMB_BYTES,
            SecretField::Breadcrumb,
        )?;
        Ok(Self {
            account_id,
            vault_id,
            item_id,
            section_id,
            field_id,
            breadcrumb,
        })
    }

    #[must_use]
    pub const fn account_id(&self) -> &OnePasswordObjectId {
        &self.account_id
    }
    #[must_use]
    pub const fn vault_id(&self) -> &OnePasswordObjectId {
        &self.vault_id
    }
    #[must_use]
    pub const fn item_id(&self) -> &OnePasswordObjectId {
        &self.item_id
    }
    #[must_use]
    pub const fn section_id(&self) -> Option<&OnePasswordSegment> {
        self.section_id.as_ref()
    }
    #[must_use]
    pub const fn field_id(&self) -> &OnePasswordSegment {
        &self.field_id
    }
    #[must_use]
    pub fn breadcrumb(&self) -> &str {
        self.breadcrumb.as_str()
    }

    /// Canonical ID-based secret reference for `op read`.
    ///
    /// Format: `op://{vault}/{item}/{field}` or
    /// `op://{vault}/{item}/{section}/{field}` when a section is present.
    /// Account is selected separately via `--account`.
    #[must_use]
    pub fn secret_reference_uri(&self) -> String {
        match self.section_id.as_ref() {
            Some(section) => format!(
                "op://{}/{}/{}/{}",
                self.vault_id.as_str(),
                self.item_id.as_str(),
                section.as_str(),
                self.field_id.as_str(),
            ),
            None => format!(
                "op://{}/{}/{}",
                self.vault_id.as_str(),
                self.item_id.as_str(),
                self.field_id.as_str(),
            ),
        }
    }

    /// Space-separated editor/wire form (IDs only; never a resolved secret).
    ///
    /// Four tokens without section: `account vault item field`.
    /// Five tokens with section: `account vault item section field`.
    #[must_use]
    pub fn to_compact_wire(&self) -> String {
        match self.section_id.as_ref() {
            Some(section) => format!(
                "{} {} {} {} {}",
                self.account_id.as_str(),
                self.vault_id.as_str(),
                self.item_id.as_str(),
                section.as_str(),
                self.field_id.as_str(),
            ),
            None => format!(
                "{} {} {} {}",
                self.account_id.as_str(),
                self.vault_id.as_str(),
                self.item_id.as_str(),
                self.field_id.as_str(),
            ),
        }
    }

    /// Parse compact wire tokens; breadcrumb defaults to the field id.
    pub fn from_compact_wire(value: &str) -> Result<Self, SecretBuildError> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        match parts.as_slice() {
            [account, vault, item, field] => {
                let field_seg = OnePasswordSegment::parse(field)?;
                let breadcrumb =
                    BoundedText::copy_from_str(field, ByteLimit::new(Self::MAX_BREADCRUMB_BYTES))
                        .map_err(|_| SecretBuildError::EmptyField {
                        field: SecretField::Breadcrumb,
                    })?;
                Self::new(
                    OnePasswordObjectId::parse(account)?,
                    OnePasswordObjectId::parse(vault)?,
                    OnePasswordObjectId::parse(item)?,
                    None,
                    field_seg,
                    breadcrumb,
                )
            }
            [account, vault, item, section, field] => {
                let field_seg = OnePasswordSegment::parse(field)?;
                let breadcrumb =
                    BoundedText::copy_from_str(field, ByteLimit::new(Self::MAX_BREADCRUMB_BYTES))
                        .map_err(|_| SecretBuildError::EmptyField {
                        field: SecretField::Breadcrumb,
                    })?;
                Self::new(
                    OnePasswordObjectId::parse(account)?,
                    OnePasswordObjectId::parse(vault)?,
                    OnePasswordObjectId::parse(item)?,
                    Some(OnePasswordSegment::parse(section)?),
                    field_seg,
                    breadcrumb,
                )
            }
            _ => Err(SecretBuildError::InvalidOnePasswordCompact),
        }
    }
}

impl fmt::Debug for OnePasswordReference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OnePasswordReference")
            .field("has_section", &self.section_id.is_some())
            .field("breadcrumb_bytes", &self.breadcrumb.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct EnvironmentReference(BoundedText);

impl EnvironmentReference {
    pub const MAX_BYTES: u64 = 128;

    pub fn parse(value: &str) -> Result<Self, SecretBuildError> {
        check_nonempty_limit(value.len(), Self::MAX_BYTES, SecretField::EnvironmentName)?;
        let mut bytes = value.bytes();
        let valid_first = bytes
            .next()
            .is_some_and(|byte| byte == b'_' || byte.is_ascii_alphabetic());
        if !valid_first || !bytes.all(|byte| byte == b'_' || byte.is_ascii_alphanumeric()) {
            return Err(SecretBuildError::InvalidEnvironmentName);
        }
        Ok(Self(
            BoundedText::copy_from_str(value, ByteLimit::new(Self::MAX_BYTES))
                .expect("validated environment name length"),
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Debug for EnvironmentReference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EnvironmentReference(REDACTED)")
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct KeychainReference(BoundedBytes);

impl KeychainReference {
    pub const MAX_BYTES: u64 = 4096;

    pub fn new(value: BoundedBytes) -> Result<Self, SecretBuildError> {
        check_nonempty_limit(value.len(), Self::MAX_BYTES, SecretField::KeychainReference)?;
        Ok(Self(value))
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl fmt::Debug for KeychainReference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("KeychainReference")
            .field("byte_len", &self.0.len())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaintextAcknowledgement {
    LocalTestingOnly,
}

#[derive(PartialEq, Eq, Hash)]
pub struct DangerousPlaintext {
    value: SensitiveBytes,
    acknowledgement: PlaintextAcknowledgement,
}

impl DangerousPlaintext {
    pub const MAX_BYTES: u64 = 65_536;

    pub fn new(
        value: Vec<u8>,
        acknowledgement: PlaintextAcknowledgement,
    ) -> Result<Self, SecretBuildError> {
        let value = SensitiveBytes::new(value)?;
        Ok(Self {
            value,
            acknowledgement,
        })
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.value.as_slice()
    }
    #[must_use]
    pub const fn acknowledgement(&self) -> PlaintextAcknowledgement {
        self.acknowledgement
    }

    pub fn clear(&mut self) {
        self.value.clear();
    }
}

#[derive(PartialEq, Eq, Hash)]
struct SensitiveBytes(Vec<u8>);

impl SensitiveBytes {
    fn new(mut value: Vec<u8>) -> Result<Self, SecretBuildError> {
        if let Err(error) = check_nonempty_limit(
            value.len(),
            DangerousPlaintext::MAX_BYTES,
            SecretField::DangerousPlaintext,
        ) {
            wipe_bytes(&mut value);
            return Err(error);
        }
        Ok(Self(value))
    }

    fn as_slice(&self) -> &[u8] {
        &self.0
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn clear(&mut self) {
        wipe_bytes(&mut self.0);
    }
}

impl Drop for SensitiveBytes {
    fn drop(&mut self) {
        self.clear();
    }
}

fn wipe_bytes(bytes: &mut Vec<u8>) {
    bytes.zeroize();
}

impl fmt::Debug for DangerousPlaintext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DangerousPlaintext")
            .field("byte_len", &self.value.len())
            .field("acknowledgement", &self.acknowledgement)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum SecretSourceKind {
    OnePassword(OnePasswordReference),
    PromptOnConnect,
    HostEnvironment(EnvironmentReference),
    Keychain(KeychainReference),
    DangerousPlaintext(DangerousPlaintext),
}

#[derive(PartialEq, Eq, Hash)]
pub struct SecretSource {
    schema_version: u16,
    kind: SecretSourceKind,
}

impl SecretSource {
    pub const SCHEMA_VERSION: u16 = 1;

    #[must_use]
    pub const fn new(kind: SecretSourceKind) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            kind,
        }
    }

    pub fn from_wire(
        schema_version: u16,
        kind: SecretSourceKind,
    ) -> Result<Self, SecretBuildError> {
        if schema_version != Self::SCHEMA_VERSION {
            return Err(SecretBuildError::UnsupportedSchemaVersion {
                actual: schema_version,
                supported: Self::SCHEMA_VERSION,
            });
        }
        Ok(Self {
            schema_version,
            kind,
        })
    }

    #[must_use]
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }
    #[must_use]
    pub const fn kind(&self) -> &SecretSourceKind {
        &self.kind
    }
    #[must_use]
    pub const fn persistence_risk(&self) -> SecretPersistenceRisk {
        match self.kind {
            SecretSourceKind::OnePassword(_)
            | SecretSourceKind::HostEnvironment(_)
            | SecretSourceKind::Keychain(_) => SecretPersistenceRisk::ReferenceOnly,
            SecretSourceKind::PromptOnConnect => SecretPersistenceRisk::Prompt,
            SecretSourceKind::DangerousPlaintext(_) => SecretPersistenceRisk::DangerousPlaintext,
        }
    }
    #[must_use]
    pub const fn requires_native_adapter(&self) -> bool {
        matches!(self.kind, SecretSourceKind::Keychain(_))
    }
}

impl fmt::Debug for SecretSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.kind {
            SecretSourceKind::OnePassword(_) => "OnePassword",
            SecretSourceKind::PromptOnConnect => "PromptOnConnect",
            SecretSourceKind::HostEnvironment(_) => "HostEnvironment",
            SecretSourceKind::Keychain(_) => "Keychain",
            SecretSourceKind::DangerousPlaintext(_) => "DangerousPlaintext",
        };
        formatter
            .debug_struct("SecretSource")
            .field("schema_version", &self.schema_version)
            .field("kind", &kind)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecretPersistenceRisk {
    ReferenceOnly,
    Prompt,
    DangerousPlaintext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecretField {
    OnePasswordSegment,
    Breadcrumb,
    EnvironmentName,
    KeychainReference,
    DangerousPlaintext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretBuildError {
    UnsupportedSchemaVersion {
        actual: u16,
        supported: u16,
    },
    InvalidObjectIdLength {
        actual: u64,
        expected: u64,
    },
    InvalidReferenceCharacter {
        byte_index: u64,
    },
    InvalidEnvironmentName,
    /// Compact wire must be 4 or 5 whitespace-separated ID tokens.
    InvalidOnePasswordCompact,
    EmptyField {
        field: SecretField,
    },
    FieldTooLong {
        field: SecretField,
        actual: u64,
        max: u64,
    },
}

impl fmt::Display for SecretBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedSchemaVersion { .. } => "unsupported secret source schema version",
            Self::InvalidObjectIdLength { .. } => "invalid 1Password object ID length",
            Self::InvalidReferenceCharacter { .. } => "invalid 1Password reference character",
            Self::InvalidEnvironmentName => "invalid host environment variable name",
            Self::InvalidOnePasswordCompact => {
                "invalid 1Password compact reference (need account vault item [section] field)"
            }
            Self::EmptyField { .. } => "secret source field is empty",
            Self::FieldTooLong { .. } => "secret source field exceeds its byte limit",
        })
    }
}

impl Error for SecretBuildError {}

fn check_nonempty_limit(
    length: usize,
    max: u64,
    field: SecretField,
) -> Result<(), SecretBuildError> {
    let actual = length as u64;
    if actual == 0 {
        Err(SecretBuildError::EmptyField { field })
    } else if actual > max {
        Err(SecretBuildError::FieldTooLong { field, actual, max })
    } else {
        Ok(())
    }
}
