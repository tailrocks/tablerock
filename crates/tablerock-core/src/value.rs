use std::{error::Error, fmt};

use crate::Revision;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Engine {
    PostgreSql,
    ClickHouse,
    Redis,
}

macro_rules! capabilities {
    ($($capability:ident),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Capability {
            $($capability),+
        }

        impl Capability {
            pub const ALL: [Self; capabilities!(@count $($capability),+)] = [
                $(Self::$capability),+
            ];

            const fn index(self) -> usize {
                self as usize
            }
        }
    };
    (@count $($capability:ident),+) => {
        <[()]>::len(&[$(capabilities!(@unit $capability)),+])
    };
    (@unit $capability:ident) => { () };
}

capabilities!(
    CatalogDatabases,
    CatalogSchemas,
    CatalogRelations,
    SqlExecution,
    RedisCommands,
    Transactions,
    EditableRows,
    BatchInsert,
    AsyncMutations,
    LogicalDatabases,
    KeyTtl,
    CurrentServerOverview,
    ServerCancellation,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnsupportedReason {
    NotApplicable,
    ServerVersion,
    Permission,
    DriverGap,
    Deployment,
    ProtocolSemantics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Availability {
    Unassessed,
    Supported,
    Unsupported(UnsupportedReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityEngineMismatch;

impl fmt::Display for CapabilityEngineMismatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("capability fact engine does not match its snapshot")
    }
}

impl Error for CapabilityEngineMismatch {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilitySnapshot {
    engine: Engine,
    revision: Revision,
    availability: [Availability; Capability::ALL.len()],
}

impl CapabilitySnapshot {
    #[must_use]
    pub const fn unassessed(engine: Engine, revision: Revision) -> Self {
        Self {
            engine,
            revision,
            availability: [Availability::Unassessed; Capability::ALL.len()],
        }
    }

    #[must_use]
    pub const fn engine(&self) -> Engine {
        self.engine
    }

    #[must_use]
    pub const fn revision(&self) -> Revision {
        self.revision
    }

    #[must_use]
    pub const fn availability(&self, capability: Capability) -> Availability {
        self.availability[capability.index()]
    }

    pub const fn with_fact(
        mut self,
        fact: CapabilityFact,
    ) -> Result<Self, CapabilityEngineMismatch> {
        if self.engine as u8 != fact.engine as u8 {
            return Err(CapabilityEngineMismatch);
        }
        self.availability[fact.capability.index()] = fact.availability;
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilityFact {
    engine: Engine,
    capability: Capability,
    availability: Availability,
}

impl CapabilityFact {
    #[must_use]
    pub const fn supported(engine: Engine, capability: Capability) -> Self {
        Self {
            engine,
            capability,
            availability: Availability::Supported,
        }
    }

    #[must_use]
    pub const fn unsupported(
        engine: Engine,
        capability: Capability,
        reason: UnsupportedReason,
    ) -> Self {
        Self {
            engine,
            capability,
            availability: Availability::Unsupported(reason),
        }
    }

    #[must_use]
    pub const fn engine(self) -> Engine {
        self.engine
    }

    #[must_use]
    pub const fn capability(self) -> Capability {
        self.capability
    }

    #[must_use]
    pub const fn availability(self) -> Availability {
        self.availability
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteLimit(u64);

impl ByteLimit {
    #[must_use]
    pub const fn new(bytes: u64) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueBuildError {
    ByteLimitExceeded { actual: u64, limit: u64 },
    InvalidTruncationLength { stored: u64, original: u64 },
}

impl fmt::Display for ValueBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ByteLimitExceeded { actual, limit } => {
                write!(
                    formatter,
                    "value contains {actual} bytes, exceeding the {limit}-byte limit"
                )
            }
            Self::InvalidTruncationLength { stored, original } => write!(
                formatter,
                "truncated value stores {stored} bytes but original length is {original}"
            ),
        }
    }
}

impl Error for ValueBuildError {}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BoundedBytes(Vec<u8>);

impl BoundedBytes {
    pub fn copy_from_slice(bytes: &[u8], limit: ByteLimit) -> Result<Self, ValueBuildError> {
        check_limit(bytes.len(), limit)?;
        Ok(Self(bytes.to_vec()))
    }

    pub fn from_vec(bytes: Vec<u8>, limit: ByteLimit) -> Result<Self, BoundedBytesError> {
        match check_limit(bytes.len(), limit) {
            Ok(()) => Ok(Self(bytes)),
            Err(kind) => Err(BoundedBytesError { kind, bytes }),
        }
    }

    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for BoundedBytes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundedBytes")
            .field("byte_len", &self.0.len())
            .finish()
    }
}

pub struct BoundedBytesError {
    kind: ValueBuildError,
    bytes: Vec<u8>,
}

impl BoundedBytesError {
    #[must_use]
    pub const fn kind(&self) -> ValueBuildError {
        self.kind
    }

    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl fmt::Debug for BoundedBytesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundedBytesError")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for BoundedBytesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(formatter)
    }
}

impl Error for BoundedBytesError {}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BoundedText(String);

impl BoundedText {
    pub fn copy_from_str(text: &str, limit: ByteLimit) -> Result<Self, ValueBuildError> {
        check_limit(text.len(), limit)?;
        Ok(Self(text.to_owned()))
    }

    pub fn from_string(text: String, limit: ByteLimit) -> Result<Self, BoundedTextError> {
        match check_limit(text.len(), limit) {
            Ok(()) => Ok(Self(text)),
            Err(kind) => Err(BoundedTextError { kind, text }),
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn allocation_capacity(&self) -> usize {
        self.0.capacity()
    }
}

impl fmt::Debug for BoundedText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundedText")
            .field("byte_len", &self.0.len())
            .finish()
    }
}

pub struct BoundedTextError {
    kind: ValueBuildError,
    text: String,
}

impl BoundedTextError {
    #[must_use]
    pub const fn kind(&self) -> ValueBuildError {
        self.kind
    }

    #[must_use]
    pub fn into_string(self) -> String {
        self.text
    }
}

impl fmt::Debug for BoundedTextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundedTextError")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for BoundedTextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(formatter)
    }
}

impl Error for BoundedTextError {}

fn check_limit(length: usize, limit: ByteLimit) -> Result<(), ValueBuildError> {
    let actual = portable_byte_len(length);
    if actual > limit.get() {
        Err(ValueBuildError::ByteLimitExceeded {
            actual,
            limit: limit.get(),
        })
    } else {
        Ok(())
    }
}

const fn portable_byte_len(length: usize) -> u64 {
    if length > u64::MAX as usize {
        u64::MAX
    } else {
        length as u64
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct EngineType {
    engine: Engine,
    name: BoundedText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmptyEngineType;

impl fmt::Display for EmptyEngineType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("engine type name must not be empty")
    }
}

impl Error for EmptyEngineType {}

impl EngineType {
    pub fn new(engine: Engine, name: BoundedText) -> Result<Self, EmptyEngineType> {
        if name.is_empty() {
            Err(EmptyEngineType)
        } else {
            Ok(Self { engine, name })
        }
    }

    #[must_use]
    pub const fn engine(&self) -> Engine {
        self.engine
    }

    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub(crate) fn allocation_capacity(&self) -> usize {
        self.name.allocation_capacity()
    }
}

impl fmt::Debug for EngineType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EngineType")
            .field("engine", &self.engine)
            .field("name", &self.name)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Truncation {
    Complete,
    Truncated { original_byte_len: Option<u64> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueKind {
    Null,
    Boolean,
    Signed,
    Unsigned,
    Float64,
    Decimal,
    Text,
    Structured,
    Binary,
    Invalid,
    Unknown,
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum ValueData {
    Null,
    Boolean(bool),
    Signed(i64),
    Unsigned(u64),
    Float64(u64),
    Decimal(BoundedText),
    Text {
        value: BoundedText,
        truncation: Truncation,
    },
    Structured {
        value: BoundedText,
        truncation: Truncation,
    },
    Binary {
        value: BoundedBytes,
        truncation: Truncation,
    },
    Invalid {
        engine_type: EngineType,
        payload: BoundedBytes,
        truncation: Truncation,
    },
    Unknown {
        engine_type: EngineType,
        payload: BoundedBytes,
        truncation: Truncation,
    },
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct OwnedValue(ValueData);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ValueRef<'a> {
    Null,
    Boolean(bool),
    Signed(i64),
    Unsigned(u64),
    Float64Bits(u64),
    Decimal(&'a str),
    Text {
        value: &'a str,
        truncation: Truncation,
    },
    Structured {
        value: &'a str,
        truncation: Truncation,
    },
    Binary {
        value: &'a [u8],
        truncation: Truncation,
    },
    Invalid {
        engine_type: &'a EngineType,
        payload: &'a [u8],
        truncation: Truncation,
    },
    Unknown {
        engine_type: &'a EngineType,
        payload: &'a [u8],
        truncation: Truncation,
    },
}

impl OwnedValue {
    #[must_use]
    pub const fn null() -> Self {
        Self(ValueData::Null)
    }

    #[must_use]
    pub const fn boolean(value: bool) -> Self {
        Self(ValueData::Boolean(value))
    }

    #[must_use]
    pub const fn signed(value: i64) -> Self {
        Self(ValueData::Signed(value))
    }

    #[must_use]
    pub const fn unsigned(value: u64) -> Self {
        Self(ValueData::Unsigned(value))
    }

    /// Stores the exact IEEE 754 payload, preserving signed zero and NaN payloads.
    #[must_use]
    pub const fn float64_bits(bits: u64) -> Self {
        Self(ValueData::Float64(bits))
    }

    #[must_use]
    pub const fn decimal(value: BoundedText) -> Self {
        Self(ValueData::Decimal(value))
    }

    pub fn text(value: BoundedText, truncation: Truncation) -> Result<Self, ValueBuildError> {
        validate_truncation(value.len(), truncation)?;
        Ok(Self(ValueData::Text { value, truncation }))
    }

    /// Stores a bounded canonical projection of a database-native container.
    pub fn structured(value: BoundedText, truncation: Truncation) -> Result<Self, ValueBuildError> {
        validate_truncation(value.len(), truncation)?;
        Ok(Self(ValueData::Structured { value, truncation }))
    }

    pub fn binary(value: BoundedBytes, truncation: Truncation) -> Result<Self, ValueBuildError> {
        validate_truncation(value.len(), truncation)?;
        Ok(Self(ValueData::Binary { value, truncation }))
    }

    pub fn invalid(
        engine_type: EngineType,
        payload: BoundedBytes,
        truncation: Truncation,
    ) -> Result<Self, ValueBuildError> {
        validate_truncation(payload.len(), truncation)?;
        Ok(Self(ValueData::Invalid {
            engine_type,
            payload,
            truncation,
        }))
    }

    pub fn unknown(
        engine_type: EngineType,
        payload: BoundedBytes,
        truncation: Truncation,
    ) -> Result<Self, ValueBuildError> {
        validate_truncation(payload.len(), truncation)?;
        Ok(Self(ValueData::Unknown {
            engine_type,
            payload,
            truncation,
        }))
    }

    #[must_use]
    pub const fn engine_type(&self) -> Option<&EngineType> {
        match &self.0 {
            ValueData::Invalid { engine_type, .. } | ValueData::Unknown { engine_type, .. } => {
                Some(engine_type)
            }
            _ => None,
        }
    }

    #[must_use]
    pub fn as_ref(&self) -> ValueRef<'_> {
        match &self.0 {
            ValueData::Null => ValueRef::Null,
            ValueData::Boolean(value) => ValueRef::Boolean(*value),
            ValueData::Signed(value) => ValueRef::Signed(*value),
            ValueData::Unsigned(value) => ValueRef::Unsigned(*value),
            ValueData::Float64(bits) => ValueRef::Float64Bits(*bits),
            ValueData::Decimal(value) => ValueRef::Decimal(value.as_str()),
            ValueData::Text { value, truncation } => ValueRef::Text {
                value: value.as_str(),
                truncation: *truncation,
            },
            ValueData::Structured { value, truncation } => ValueRef::Structured {
                value: value.as_str(),
                truncation: *truncation,
            },
            ValueData::Binary { value, truncation } => ValueRef::Binary {
                value: value.as_slice(),
                truncation: *truncation,
            },
            ValueData::Invalid {
                engine_type,
                payload,
                truncation,
            } => ValueRef::Invalid {
                engine_type,
                payload: payload.as_slice(),
                truncation: *truncation,
            },
            ValueData::Unknown {
                engine_type,
                payload,
                truncation,
            } => ValueRef::Unknown {
                engine_type,
                payload: payload.as_slice(),
                truncation: *truncation,
            },
        }
    }

    #[must_use]
    pub const fn kind(&self) -> ValueKind {
        match self.0 {
            ValueData::Null => ValueKind::Null,
            ValueData::Boolean(_) => ValueKind::Boolean,
            ValueData::Signed(_) => ValueKind::Signed,
            ValueData::Unsigned(_) => ValueKind::Unsigned,
            ValueData::Float64(_) => ValueKind::Float64,
            ValueData::Decimal(_) => ValueKind::Decimal,
            ValueData::Text { .. } => ValueKind::Text,
            ValueData::Structured { .. } => ValueKind::Structured,
            ValueData::Binary { .. } => ValueKind::Binary,
            ValueData::Invalid { .. } => ValueKind::Invalid,
            ValueData::Unknown { .. } => ValueKind::Unknown,
        }
    }

    #[must_use]
    pub const fn is_truncated(&self) -> bool {
        matches!(
            self.0,
            ValueData::Text {
                truncation: Truncation::Truncated { .. },
                ..
            } | ValueData::Structured {
                truncation: Truncation::Truncated { .. },
                ..
            } | ValueData::Binary {
                truncation: Truncation::Truncated { .. },
                ..
            } | ValueData::Invalid {
                truncation: Truncation::Truncated { .. },
                ..
            } | ValueData::Unknown {
                truncation: Truncation::Truncated { .. },
                ..
            }
        )
    }

    #[must_use]
    pub fn encoded_byte_len(&self) -> u64 {
        match self.as_ref() {
            ValueRef::Null => 0,
            ValueRef::Boolean(_) => 1,
            ValueRef::Signed(_) | ValueRef::Unsigned(_) | ValueRef::Float64Bits(_) => 8,
            ValueRef::Decimal(value)
            | ValueRef::Text { value, .. }
            | ValueRef::Structured { value, .. } => portable_byte_len(value.len()),
            ValueRef::Binary { value, .. }
            | ValueRef::Invalid { payload: value, .. }
            | ValueRef::Unknown { payload: value, .. } => portable_byte_len(value.len()),
        }
    }
}

impl fmt::Debug for OwnedValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OwnedValue")
            .field("kind", &self.kind())
            .field("truncated", &self.is_truncated())
            .finish_non_exhaustive()
    }
}

fn validate_truncation(length: usize, truncation: Truncation) -> Result<(), ValueBuildError> {
    if let Truncation::Truncated {
        original_byte_len: Some(original),
    } = truncation
    {
        let stored = portable_byte_len(length);
        if original <= stored {
            return Err(ValueBuildError::InvalidTruncationLength { stored, original });
        }
    }
    Ok(())
}
