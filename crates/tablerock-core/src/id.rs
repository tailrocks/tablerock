use std::{error::Error, fmt, num::NonZeroU128, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdDecodeError {
    Zero,
    InvalidLength,
    InvalidHex { index: u8 },
}

impl fmt::Display for IdDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => formatter.write_str("identifier must be nonzero"),
            Self::InvalidLength => {
                formatter.write_str("identifier must contain exactly 32 hex digits")
            }
            Self::InvalidHex { index } => {
                write!(formatter, "identifier has invalid hex at byte {index}")
            }
        }
    }
}

impl Error for IdDecodeError {}

/// Canonical FFI-safe identity payload. Numeric and byte encodings are big-endian.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdParts {
    pub high: u64,
    pub low: u64,
}

impl IdParts {
    pub const fn new(high: u64, low: u64) -> Result<Self, IdDecodeError> {
        if high == 0 && low == 0 {
            Err(IdDecodeError::Zero)
        } else {
            Ok(Self { high, low })
        }
    }

    #[must_use]
    pub const fn to_bytes(self) -> [u8; 16] {
        let high = self.high.to_be_bytes();
        let low = self.low.to_be_bytes();
        [
            high[0], high[1], high[2], high[3], high[4], high[5], high[6], high[7], low[0], low[1],
            low[2], low[3], low[4], low[5], low[6], low[7],
        ]
    }

    pub const fn from_bytes(bytes: [u8; 16]) -> Result<Self, IdDecodeError> {
        let high = u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        let low = u64::from_be_bytes([
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]);
        Self::new(high, low)
    }

    const fn as_nonzero(self) -> Result<NonZeroU128, IdDecodeError> {
        match NonZeroU128::new(((self.high as u128) << 64) | self.low as u128) {
            Some(value) => Ok(value),
            None => Err(IdDecodeError::Zero),
        }
    }

    const fn from_nonzero(value: NonZeroU128) -> Self {
        let value = value.get();
        Self {
            high: (value >> 64) as u64,
            low: value as u64,
        }
    }
}

fn parse_parts(text: &str) -> Result<IdParts, IdDecodeError> {
    if text.len() != 32 {
        return Err(IdDecodeError::InvalidLength);
    }
    let mut bytes = [0_u8; 16];
    for (index, pair) in text.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_nibble(pair[0]).ok_or(IdDecodeError::InvalidHex {
            index: (index * 2) as u8,
        })?;
        let low = hex_nibble(pair[1]).ok_or(IdDecodeError::InvalidHex {
            index: (index * 2 + 1) as u8,
        })?;
        bytes[index] = (high << 4) | low;
    }
    IdParts::from_bytes(bytes)
}

const fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

macro_rules! opaque_ids {
    ($($name:ident),+ $(,)?) => {
        $(
            #[repr(transparent)]
            #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct $name(NonZeroU128);

            impl $name {
                pub const fn from_parts(parts: IdParts) -> Result<Self, IdDecodeError> {
                    match parts.as_nonzero() {
                        Ok(value) => Ok(Self(value)),
                        Err(error) => Err(error),
                    }
                }

                pub const fn from_bytes(bytes: [u8; 16]) -> Result<Self, IdDecodeError> {
                    match IdParts::from_bytes(bytes) {
                        Ok(parts) => Self::from_parts(parts),
                        Err(error) => Err(error),
                    }
                }

                #[must_use]
                pub const fn parts(self) -> IdParts {
                    IdParts::from_nonzero(self.0)
                }

                #[must_use]
                pub const fn to_bytes(self) -> [u8; 16] {
                    self.parts().to_bytes()
                }
            }

            impl FromStr for $name {
                type Err = IdDecodeError;

                fn from_str(text: &str) -> Result<Self, Self::Err> {
                    match parse_parts(text) {
                        Ok(parts) => Self::from_parts(parts),
                        Err(error) => Err(error),
                    }
                }
            }

            impl fmt::Debug for $name {
                fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    write!(formatter, "{}({self})", stringify!($name))
                }
            }

            impl fmt::Display for $name {
                fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    let parts = self.parts();
                    write!(formatter, "{:016x}{:016x}", parts.high, parts.low)
                }
            }
        )+
    };
}

opaque_ids!(
    ProfileId,
    SessionId,
    ContextId,
    TabId,
    QueryId,
    ResultId,
    RowId,
    MutationId,
    OperationId,
    RequestId,
    CatalogNodeId,
    ReviewTokenId,
    SubscriptionId,
);
