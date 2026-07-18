use std::{error::Error, fmt};

use crate::{Engine, EngineType, OwnedValue, ResultId, Revision, Truncation, ValueKind, ValueRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RowTotal {
    Unknown,
    Known(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageDelivery {
    Partial,
    Final,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageWarning {
    RowLimitReached,
    ByteLimitReached,
    UnknownValues,
    InvalidValues,
    PartialFailure,
    DeliveryDiscontinuity,
}

impl PageWarning {
    const COUNT: usize = 6;

    const fn index(self) -> usize {
        match self {
            Self::RowLimitReached => 0,
            Self::ByteLimitReached => 1,
            Self::UnknownValues => 2,
            Self::InvalidValues => 3,
            Self::PartialFailure => 4,
            Self::DeliveryDiscontinuity => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PageWarnings(u16);

const _: () = assert!(PageWarning::COUNT <= u16::BITS as usize);

impl PageWarnings {
    /// Bits past the documented warning set are reserved for future versions.
    const RESERVED_MASK: u16 = !((1 << PageWarning::COUNT) - 1);

    #[must_use]
    pub const fn none() -> Self {
        Self(0)
    }

    #[must_use]
    pub const fn with(self, warning: PageWarning) -> Self {
        Self(self.0 | (1 << warning.index()))
    }

    #[must_use]
    pub const fn contains(self, warning: PageWarning) -> bool {
        self.0 & (1 << warning.index()) != 0
    }

    #[must_use]
    pub const fn bits(self) -> u16 {
        self.0
    }

    pub const fn from_bits(bits: u16) -> Result<Self, PageValidationError> {
        if bits & Self::RESERVED_MASK != 0 {
            return Err(PageValidationError::ReservedWarningBits);
        }
        Ok(Self(bits))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageIdentity {
    result_id: ResultId,
    revision: Revision,
    engine: Engine,
}

impl PageIdentity {
    #[must_use]
    pub const fn new(result_id: ResultId, revision: Revision, engine: Engine) -> Self {
        Self {
            result_id,
            revision,
            engine,
        }
    }

    #[must_use]
    pub const fn result_id(self) -> ResultId {
        self.result_id
    }

    #[must_use]
    pub const fn revision(self) -> Revision {
        self.revision
    }

    #[must_use]
    pub const fn engine(self) -> Engine {
        self.engine
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageShape {
    start_row: u64,
    row_count: u32,
    column_count: u32,
    total_rows: RowTotal,
    arena_byte_len: u64,
    column_text_byte_len: u64,
}

impl PageShape {
    #[must_use]
    pub const fn new(
        start_row: u64,
        row_count: u32,
        column_count: u32,
        total_rows: RowTotal,
        arena_byte_len: u64,
        column_text_byte_len: u64,
    ) -> Self {
        Self {
            start_row,
            row_count,
            column_count,
            total_rows,
            arena_byte_len,
            column_text_byte_len,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageFacts {
    delivery: PageDelivery,
    warnings: PageWarnings,
}

impl PageFacts {
    #[must_use]
    pub const fn new(delivery: PageDelivery, warnings: PageWarnings) -> Self {
        Self { delivery, warnings }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageLimits {
    max_rows: u32,
    max_columns: u32,
    max_arena_bytes: u64,
    max_column_text_bytes: u64,
}

impl PageLimits {
    #[must_use]
    pub const fn new(
        max_rows: u32,
        max_columns: u32,
        max_arena_bytes: u64,
        max_column_text_bytes: u64,
    ) -> Self {
        Self {
            max_rows,
            max_columns,
            max_arena_bytes,
            max_column_text_bytes,
        }
    }

    #[must_use]
    pub const fn max_rows(self) -> u32 {
        self.max_rows
    }

    #[must_use]
    pub const fn max_columns(self) -> u32 {
        self.max_columns
    }

    #[must_use]
    pub const fn max_arena_bytes(self) -> u64 {
        self.max_arena_bytes
    }

    #[must_use]
    pub const fn max_column_text_bytes(self) -> u64 {
        self.max_column_text_bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageValidationError {
    UnsupportedEncodingVersion {
        actual: u16,
        supported: u16,
    },
    RowLimitExceeded {
        actual: u32,
        limit: u32,
    },
    ColumnLimitExceeded {
        actual: u32,
        limit: u32,
    },
    ArenaLimitExceeded {
        actual: u64,
        limit: u64,
    },
    ColumnTextLimitExceeded {
        actual: u64,
        limit: u64,
    },
    RowRangeOverflow,
    KnownTotalBeforePageEnd {
        total: u64,
        page_end: u64,
    },
    RowsWithoutColumns,
    CellCountUnsupported {
        cells: u64,
    },
    CellCountMismatch {
        expected: u64,
        actual: u64,
    },
    ArenaLengthOverflow,
    ArenaLengthUnsupported {
        bytes: u64,
    },
    ColumnCountMismatch {
        expected: u32,
        actual: u64,
    },
    ColumnEngineMismatch {
        column: u32,
    },
    ColumnTextLengthMismatch {
        declared: u64,
        actual: u64,
    },
    ColumnTextLengthOverflow,
    ArenaLengthMismatch {
        declared: u64,
        actual: u64,
    },
    CellOffsetCountMismatch {
        expected: u64,
        actual: u64,
    },
    KindCountMismatch {
        expected: u64,
        actual: u64,
    },
    TruncationCountMismatch {
        expected: u64,
        actual: u64,
    },
    NullBitmapLengthMismatch {
        expected: u64,
        actual: u64,
    },
    NonzeroNullPadding,
    FirstOffsetNotZero {
        actual: u64,
    },
    OffsetDecreases {
        cell: u64,
    },
    OffsetOutsideArena {
        cell: u64,
        offset: u64,
        arena_bytes: u64,
    },
    FinalOffsetMismatch {
        offset: u64,
        arena_bytes: u64,
    },
    NullKindMismatch {
        cell: u64,
    },
    NullHasBytes {
        cell: u64,
    },
    NullIsTruncated {
        cell: u64,
    },
    NullInNonNullableColumn {
        cell: u64,
        column: u32,
    },
    InvalidBooleanEncoding {
        cell: u64,
    },
    InvalidFixedWidthEncoding {
        cell: u64,
        expected: u64,
        actual: u64,
    },
    InvalidUtf8Encoding {
        cell: u64,
    },
    UnsupportedTruncationKind {
        cell: u64,
        kind: ValueKind,
    },
    InvalidTruncationLength {
        cell: u64,
        stored: u64,
        original: u64,
    },
    TruncatedEncoding,
    InvalidMagic,
    InvalidIdentity,
    InvalidTotalRowsTag,
    InvalidDeliveryTag,
    InvalidNullableTag,
    InvalidValueKindTag,
    InvalidTruncationTag,
    InvalidEngineTag,
    EmptyEngineTypeName,
    TrailingBytes,
    ReservedWarningBits,
}

impl fmt::Display for PageValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid result page: {self:?}")
    }
}

impl Error for PageValidationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageEnvelope {
    encoding_version: u16,
    result_id: ResultId,
    revision: Revision,
    engine: Engine,
    start_row: u64,
    row_count: u32,
    column_count: u32,
    total_rows: RowTotal,
    arena_byte_len: u64,
    column_text_byte_len: u64,
    delivery: PageDelivery,
    warnings: PageWarnings,
}

impl PageEnvelope {
    pub const ENCODING_VERSION: u16 = 1;

    #[must_use]
    pub const fn new(identity: PageIdentity, shape: PageShape, facts: PageFacts) -> Self {
        Self {
            encoding_version: Self::ENCODING_VERSION,
            result_id: identity.result_id,
            revision: identity.revision,
            engine: identity.engine,
            start_row: shape.start_row,
            row_count: shape.row_count,
            column_count: shape.column_count,
            total_rows: shape.total_rows,
            arena_byte_len: shape.arena_byte_len,
            column_text_byte_len: shape.column_text_byte_len,
            delivery: facts.delivery,
            warnings: facts.warnings,
        }
    }

    /// Restores a wire version for validation before any page buffers are allocated.
    #[must_use]
    pub const fn from_wire(
        encoding_version: u16,
        identity: PageIdentity,
        shape: PageShape,
        facts: PageFacts,
    ) -> Self {
        let mut envelope = Self::new(identity, shape, facts);
        envelope.encoding_version = encoding_version;
        envelope
    }

    pub const fn validate(
        self,
        limits: PageLimits,
    ) -> Result<ValidatedPageEnvelope, PageValidationError> {
        if self.encoding_version != Self::ENCODING_VERSION {
            return Err(PageValidationError::UnsupportedEncodingVersion {
                actual: self.encoding_version,
                supported: Self::ENCODING_VERSION,
            });
        }
        if self.row_count > limits.max_rows {
            return Err(PageValidationError::RowLimitExceeded {
                actual: self.row_count,
                limit: limits.max_rows,
            });
        }
        if self.column_count > limits.max_columns {
            return Err(PageValidationError::ColumnLimitExceeded {
                actual: self.column_count,
                limit: limits.max_columns,
            });
        }
        if self.arena_byte_len > limits.max_arena_bytes {
            return Err(PageValidationError::ArenaLimitExceeded {
                actual: self.arena_byte_len,
                limit: limits.max_arena_bytes,
            });
        }
        if self.column_text_byte_len > limits.max_column_text_bytes {
            return Err(PageValidationError::ColumnTextLimitExceeded {
                actual: self.column_text_byte_len,
                limit: limits.max_column_text_bytes,
            });
        }
        if self.row_count > 0 && self.column_count == 0 {
            return Err(PageValidationError::RowsWithoutColumns);
        }
        let Some(page_end) = self.start_row.checked_add(self.row_count as u64) else {
            return Err(PageValidationError::RowRangeOverflow);
        };
        if let RowTotal::Known(total) = self.total_rows
            && total < page_end
        {
            return Err(PageValidationError::KnownTotalBeforePageEnd { total, page_end });
        }
        let cells = self.row_count as u64 * self.column_count as u64;
        if cells > usize::MAX as u64 {
            return Err(PageValidationError::CellCountUnsupported { cells });
        }
        Ok(ValidatedPageEnvelope(self))
    }

    #[must_use]
    pub const fn result_id(self) -> ResultId {
        self.result_id
    }

    #[must_use]
    pub const fn revision(self) -> Revision {
        self.revision
    }

    #[must_use]
    pub const fn engine(self) -> Engine {
        self.engine
    }

    #[must_use]
    pub const fn start_row(self) -> u64 {
        self.start_row
    }

    #[must_use]
    pub const fn row_count(self) -> u32 {
        self.row_count
    }

    #[must_use]
    pub const fn column_count(self) -> u32 {
        self.column_count
    }

    #[must_use]
    pub const fn arena_byte_len(self) -> u64 {
        self.arena_byte_len
    }

    #[must_use]
    pub const fn column_text_byte_len(self) -> u64 {
        self.column_text_byte_len
    }

    #[must_use]
    pub const fn total_rows(self) -> RowTotal {
        self.total_rows
    }

    #[must_use]
    pub const fn delivery(self) -> PageDelivery {
        self.delivery
    }

    #[must_use]
    pub const fn warnings(self) -> PageWarnings {
        self.warnings
    }
}

/// Proof that the cheap envelope checks passed before owned page buffers are accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValidatedPageEnvelope(PageEnvelope);

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ColumnMetadata {
    name: crate::BoundedText,
    engine_type: EngineType,
    nullable: bool,
}

impl ColumnMetadata {
    #[must_use]
    pub const fn new(name: crate::BoundedText, engine_type: EngineType, nullable: bool) -> Self {
        Self {
            name,
            engine_type,
            nullable,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    #[must_use]
    pub const fn engine_type(&self) -> &EngineType {
        &self.engine_type
    }

    #[must_use]
    pub const fn nullable(&self) -> bool {
        self.nullable
    }

    fn column_text_byte_len(&self) -> u64 {
        self.name.len() as u64 + self.engine_type.name().len() as u64
    }

    fn allocation_capacity(&self) -> usize {
        self.name.allocation_capacity() + self.engine_type.allocation_capacity()
    }
}

impl fmt::Debug for ColumnMetadata {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ColumnMetadata")
            .field("name_bytes", &self.name.len())
            .field("engine", &self.engine_type.engine())
            .field("type_name_bytes", &self.engine_type.name().len())
            .field("nullable", &self.nullable)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct PageBuffers {
    columns: Vec<ColumnMetadata>,
    cell_offsets: Vec<u64>,
    null_bitmap: Vec<u8>,
    value_kinds: Vec<ValueKind>,
    truncations: Vec<Truncation>,
    arena: Vec<u8>,
}

impl PageBuffers {
    #[must_use]
    pub fn new(
        columns: Vec<ColumnMetadata>,
        cell_offsets: Vec<u64>,
        null_bitmap: Vec<u8>,
        value_kinds: Vec<ValueKind>,
        truncations: Vec<Truncation>,
        arena: Vec<u8>,
    ) -> Self {
        Self {
            columns,
            cell_offsets,
            null_bitmap,
            value_kinds,
            truncations,
            arena,
        }
    }
}

impl fmt::Debug for PageBuffers {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PageBuffers")
            .field("columns", &self.columns.len())
            .field("cells", &self.value_kinds.len())
            .field("arena_bytes", &self.arena.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ResultPage {
    envelope: PageEnvelope,
    buffers: PageBuffers,
}

impl ResultPage {
    /// Converts bounded row-major adapter output into the canonical immutable
    /// column-major page representation.
    pub fn from_row_major(
        identity: PageIdentity,
        start_row: u64,
        total_rows: RowTotal,
        facts: PageFacts,
        columns: Vec<ColumnMetadata>,
        values: Vec<OwnedValue>,
        limits: PageLimits,
    ) -> Result<Self, PageValidationError> {
        let column_count =
            u32::try_from(columns.len()).map_err(|_| PageValidationError::ColumnLimitExceeded {
                actual: u32::MAX,
                limit: limits.max_columns,
            })?;
        if column_count > limits.max_columns {
            return Err(PageValidationError::ColumnLimitExceeded {
                actual: column_count,
                limit: limits.max_columns,
            });
        }
        if column_count == 0 && !values.is_empty() {
            return Err(PageValidationError::CellCountMismatch {
                expected: 0,
                actual: values.len() as u64,
            });
        }
        if column_count != 0 && !values.len().is_multiple_of(column_count as usize) {
            let complete_rows = values.len() / column_count as usize;
            return Err(PageValidationError::CellCountMismatch {
                expected: (complete_rows as u64 + 1) * column_count as u64,
                actual: values.len() as u64,
            });
        }
        let row_count = if column_count == 0 {
            0
        } else {
            values.len() / column_count as usize
        };
        let row_count =
            u32::try_from(row_count).map_err(|_| PageValidationError::RowLimitExceeded {
                actual: u32::MAX,
                limit: limits.max_rows,
            })?;
        if row_count > limits.max_rows {
            return Err(PageValidationError::RowLimitExceeded {
                actual: row_count,
                limit: limits.max_rows,
            });
        }

        let arena_byte_len = values.iter().try_fold(0_u64, |total, value| {
            total.checked_add(value_byte_len(value))
        });
        let Some(arena_byte_len) = arena_byte_len else {
            return Err(PageValidationError::ArenaLengthOverflow);
        };
        let arena_capacity = usize::try_from(arena_byte_len).map_err(|_| {
            PageValidationError::ArenaLengthUnsupported {
                bytes: arena_byte_len,
            }
        })?;
        let column_text_byte_len = columns.iter().try_fold(0_u64, |total, column| {
            total.checked_add(column.column_text_byte_len())
        });
        let Some(column_text_byte_len) = column_text_byte_len else {
            return Err(PageValidationError::ColumnTextLengthOverflow);
        };
        let envelope = PageEnvelope::new(
            identity,
            PageShape::new(
                start_row,
                row_count,
                column_count,
                total_rows,
                arena_byte_len,
                column_text_byte_len,
            ),
            facts,
        );
        let validated = envelope.validate(limits)?;
        let cell_count = values.len();
        let mut offsets = Vec::with_capacity(cell_count + 1);
        let mut null_bitmap = vec![0; cell_count.div_ceil(8)];
        let mut kinds = Vec::with_capacity(cell_count);
        let mut truncations = Vec::with_capacity(cell_count);
        let mut arena = Vec::with_capacity(arena_capacity);
        offsets.push(0);
        for column in 0..column_count as usize {
            for row in 0..row_count as usize {
                let value = &values[row * column_count as usize + column];
                let cell = kinds.len();
                append_value(value, &mut arena);
                if value.kind() == ValueKind::Null {
                    null_bitmap[cell / 8] |= 1 << (cell % 8);
                }
                kinds.push(value.kind());
                truncations.push(value_truncation(value));
                offsets.push(arena.len() as u64);
            }
        }
        Self::from_parts(
            validated,
            PageBuffers::new(columns, offsets, null_bitmap, kinds, truncations, arena),
        )
    }

    pub fn from_parts(
        validated: ValidatedPageEnvelope,
        buffers: PageBuffers,
    ) -> Result<Self, PageValidationError> {
        let envelope = validated.0;
        validate_parts(envelope, &buffers)?;
        Ok(Self { envelope, buffers })
    }

    #[must_use]
    pub const fn envelope(&self) -> PageEnvelope {
        self.envelope
    }

    /// Counts all heap capacity owned by the page's immutable buffers.
    #[must_use]
    pub fn resident_buffer_bytes(&self) -> u64 {
        let buffers = &self.buffers;
        let mut bytes = buffers
            .columns
            .capacity()
            .saturating_mul(std::mem::size_of::<ColumnMetadata>());
        bytes = bytes.saturating_add(
            buffers
                .columns
                .iter()
                .map(ColumnMetadata::allocation_capacity)
                .fold(0_usize, usize::saturating_add),
        );
        bytes = bytes.saturating_add(
            buffers
                .cell_offsets
                .capacity()
                .saturating_mul(std::mem::size_of::<u64>()),
        );
        bytes = bytes.saturating_add(buffers.null_bitmap.capacity());
        bytes = bytes.saturating_add(
            buffers
                .value_kinds
                .capacity()
                .saturating_mul(std::mem::size_of::<ValueKind>()),
        );
        bytes = bytes.saturating_add(
            buffers
                .truncations
                .capacity()
                .saturating_mul(std::mem::size_of::<Truncation>()),
        );
        bytes = bytes.saturating_add(buffers.arena.capacity());
        u64::try_from(bytes).unwrap_or(u64::MAX)
    }

    #[must_use]
    pub fn columns(&self) -> &[ColumnMetadata] {
        &self.buffers.columns
    }

    pub fn cell(&self, row: u32, column: u32) -> Result<CellRef<'_>, PageAccessError> {
        if row >= self.envelope.row_count {
            return Err(PageAccessError::RowOutsidePage);
        }
        if column >= self.envelope.column_count {
            return Err(PageAccessError::ColumnOutsidePage);
        }
        let index = column as usize * self.envelope.row_count as usize + row as usize;
        let start = self.buffers.cell_offsets[index] as usize;
        let end = self.buffers.cell_offsets[index + 1] as usize;
        Ok(CellRef {
            kind: self.buffers.value_kinds[index],
            is_null: null_bit(&self.buffers.null_bitmap, index),
            truncation: self.buffers.truncations[index],
            bytes: &self.buffers.arena[start..end],
        })
    }

    /// Serializes this page as the version-1 columnar byte-arena payload.
    ///
    /// The native UniFFI bridge and conformance suite treat this encoding as
    /// the sole page wire format. Bounds in the envelope are already validated
    /// at page construction; encode is infallible for a well-formed page.
    #[must_use]
    pub fn encode_v1(&self) -> Vec<u8> {
        let envelope = self.envelope;
        let buffers = &self.buffers;
        let cells = envelope.row_count as usize * envelope.column_count as usize;
        let mut out = Vec::with_capacity(estimate_encoded_len(envelope, buffers));
        out.extend_from_slice(PAGE_V1_MAGIC);
        out.extend_from_slice(&PageEnvelope::ENCODING_VERSION.to_le_bytes());
        out.extend_from_slice(&envelope.result_id.to_bytes());
        out.extend_from_slice(&envelope.revision.get().to_le_bytes());
        out.push(engine_to_wire(envelope.engine));
        out.extend_from_slice(&envelope.start_row.to_le_bytes());
        out.extend_from_slice(&envelope.row_count.to_le_bytes());
        out.extend_from_slice(&envelope.column_count.to_le_bytes());
        match envelope.total_rows {
            RowTotal::Unknown => {
                out.push(0);
                out.extend_from_slice(&0_u64.to_le_bytes());
            }
            RowTotal::Known(total) => {
                out.push(1);
                out.extend_from_slice(&total.to_le_bytes());
            }
        }
        out.extend_from_slice(&envelope.arena_byte_len.to_le_bytes());
        out.extend_from_slice(&envelope.column_text_byte_len.to_le_bytes());
        out.push(match envelope.delivery {
            PageDelivery::Partial => 0,
            PageDelivery::Final => 1,
        });
        out.extend_from_slice(&envelope.warnings.bits().to_le_bytes());

        for column in &buffers.columns {
            write_bounded_str(&mut out, column.name());
            out.push(engine_to_wire(column.engine_type().engine()));
            write_bounded_str(&mut out, column.engine_type().name());
            out.push(u8::from(column.nullable()));
        }

        for offset in &buffers.cell_offsets {
            out.extend_from_slice(&offset.to_le_bytes());
        }
        debug_assert_eq!(buffers.cell_offsets.len(), cells + 1);
        out.extend_from_slice(&buffers.null_bitmap);
        for kind in &buffers.value_kinds {
            out.push(value_kind_to_wire(*kind));
        }
        for truncation in &buffers.truncations {
            write_truncation(&mut out, *truncation);
        }
        out.extend_from_slice(&buffers.arena);
        out
    }

    /// Decodes a version-1 page, validating the envelope against `limits`
    /// before allocating owned buffers.
    pub fn decode_v1(bytes: &[u8], limits: PageLimits) -> Result<Self, PageValidationError> {
        let mut cursor = ByteCursor::new(bytes);
        let magic = cursor
            .take(4)
            .ok_or(PageValidationError::TruncatedEncoding)?;
        if magic != PAGE_V1_MAGIC {
            return Err(PageValidationError::InvalidMagic);
        }
        let encoding_version = cursor
            .u16_le()
            .ok_or(PageValidationError::TruncatedEncoding)?;
        let result_id = ResultId::from_bytes(
            cursor
                .array16()
                .ok_or(PageValidationError::TruncatedEncoding)?,
        )
        .map_err(|_| PageValidationError::InvalidIdentity)?;
        let revision = Revision::from_wire_u64(
            cursor
                .u64_le()
                .ok_or(PageValidationError::TruncatedEncoding)?,
        );
        let engine =
            engine_from_wire(cursor.u8().ok_or(PageValidationError::TruncatedEncoding)?)?;
        let start_row = cursor
            .u64_le()
            .ok_or(PageValidationError::TruncatedEncoding)?;
        let row_count = cursor
            .u32_le()
            .ok_or(PageValidationError::TruncatedEncoding)?;
        let column_count = cursor
            .u32_le()
            .ok_or(PageValidationError::TruncatedEncoding)?;
        let total_tag = cursor.u8().ok_or(PageValidationError::TruncatedEncoding)?;
        let total_value = cursor
            .u64_le()
            .ok_or(PageValidationError::TruncatedEncoding)?;
        let total_rows = match total_tag {
            0 => {
                if total_value != 0 {
                    return Err(PageValidationError::InvalidTotalRowsTag);
                }
                RowTotal::Unknown
            }
            1 => RowTotal::Known(total_value),
            _ => return Err(PageValidationError::InvalidTotalRowsTag),
        };
        let arena_byte_len = cursor
            .u64_le()
            .ok_or(PageValidationError::TruncatedEncoding)?;
        let column_text_byte_len = cursor
            .u64_le()
            .ok_or(PageValidationError::TruncatedEncoding)?;
        let delivery = match cursor.u8().ok_or(PageValidationError::TruncatedEncoding)? {
            0 => PageDelivery::Partial,
            1 => PageDelivery::Final,
            _ => return Err(PageValidationError::InvalidDeliveryTag),
        };
        let warnings = PageWarnings::from_bits(
            cursor
                .u16_le()
                .ok_or(PageValidationError::TruncatedEncoding)?,
        )?;

        // Cheap envelope validation before any large buffer allocation.
        let envelope = PageEnvelope::from_wire(
            encoding_version,
            PageIdentity::new(result_id, revision, engine),
            PageShape::new(
                start_row,
                row_count,
                column_count,
                total_rows,
                arena_byte_len,
                column_text_byte_len,
            ),
            PageFacts::new(delivery, warnings),
        );
        let validated = envelope.validate(limits)?;

        let cells_u64 = u64::from(row_count)
            .checked_mul(u64::from(column_count))
            .ok_or(PageValidationError::CellCountUnsupported {
                cells: u64::MAX,
            })?;
        let cells = usize::try_from(cells_u64).map_err(|_| {
            PageValidationError::CellCountUnsupported {
                cells: cells_u64,
            }
        })?;

        let mut columns = Vec::with_capacity(column_count as usize);
        let mut observed_column_text = 0_u64;
        for _ in 0..column_count {
            let name_bytes = read_length_prefixed(&mut cursor)?;
            let type_engine =
                engine_from_wire(cursor.u8().ok_or(PageValidationError::TruncatedEncoding)?)?;
            let type_name_bytes = read_length_prefixed(&mut cursor)?;
            let nullable = match cursor.u8().ok_or(PageValidationError::TruncatedEncoding)? {
                0 => false,
                1 => true,
                _ => return Err(PageValidationError::InvalidNullableTag),
            };
            let name_len = name_bytes.len() as u64;
            let type_len = type_name_bytes.len() as u64;
            observed_column_text = observed_column_text
                .checked_add(name_len)
                .and_then(|total| total.checked_add(type_len))
                .ok_or(PageValidationError::ColumnTextLengthOverflow)?;
            if observed_column_text > limits.max_column_text_bytes {
                return Err(PageValidationError::ColumnTextLimitExceeded {
                    actual: observed_column_text,
                    limit: limits.max_column_text_bytes,
                });
            }
            let name = crate::BoundedText::copy_from_str(
                std::str::from_utf8(name_bytes)
                    .map_err(|_| PageValidationError::InvalidUtf8Encoding { cell: 0 })?,
                crate::ByteLimit::new(name_len.max(1)),
            )
            .map_err(|_| PageValidationError::ColumnTextLengthOverflow)?;
            let type_name = crate::BoundedText::copy_from_str(
                std::str::from_utf8(type_name_bytes)
                    .map_err(|_| PageValidationError::InvalidUtf8Encoding { cell: 0 })?,
                crate::ByteLimit::new(type_len.max(1)),
            )
            .map_err(|_| PageValidationError::ColumnTextLengthOverflow)?;
            let engine_type = EngineType::new(type_engine, type_name)
                .map_err(|_| PageValidationError::EmptyEngineTypeName)?;
            columns.push(ColumnMetadata::new(name, engine_type, nullable));
        }
        if observed_column_text != column_text_byte_len {
            return Err(PageValidationError::ColumnTextLengthMismatch {
                declared: column_text_byte_len,
                actual: observed_column_text,
            });
        }

        let offset_count = cells.checked_add(1).ok_or(PageValidationError::CellCountUnsupported {
            cells: cells_u64,
        })?;
        let mut cell_offsets = Vec::with_capacity(offset_count);
        for _ in 0..offset_count {
            cell_offsets.push(
                cursor
                    .u64_le()
                    .ok_or(PageValidationError::TruncatedEncoding)?,
            );
        }

        let bitmap_len = cells.div_ceil(8);
        let null_bitmap = cursor
            .take(bitmap_len)
            .ok_or(PageValidationError::TruncatedEncoding)?
            .to_vec();

        let mut value_kinds = Vec::with_capacity(cells);
        for _ in 0..cells {
            value_kinds.push(value_kind_from_wire(
                cursor.u8().ok_or(PageValidationError::TruncatedEncoding)?,
            )?);
        }

        let mut truncations = Vec::with_capacity(cells);
        for _ in 0..cells {
            truncations.push(read_truncation(&mut cursor)?);
        }

        let arena_len = usize::try_from(arena_byte_len).map_err(|_| {
            PageValidationError::ArenaLengthUnsupported {
                bytes: arena_byte_len,
            }
        })?;
        let arena = cursor
            .take(arena_len)
            .ok_or(PageValidationError::TruncatedEncoding)?
            .to_vec();
        if cursor.remaining() != 0 {
            return Err(PageValidationError::TrailingBytes);
        }

        let buffers = PageBuffers::new(
            columns,
            cell_offsets,
            null_bitmap,
            value_kinds,
            truncations,
            arena,
        );
        Self::from_parts(validated, buffers)
    }
}

fn value_byte_len(value: &OwnedValue) -> u64 {
    match value.as_ref() {
        ValueRef::Null => 0,
        ValueRef::Boolean(_) => 1,
        ValueRef::Signed(_) | ValueRef::Unsigned(_) | ValueRef::Float64Bits(_) => 8,
        ValueRef::Decimal(value)
        | ValueRef::Temporal { value, .. }
        | ValueRef::Text { value, .. }
        | ValueRef::Structured { value, .. } => value.len() as u64,
        ValueRef::Binary { value, .. }
        | ValueRef::Invalid { payload: value, .. }
        | ValueRef::Unknown { payload: value, .. } => value.len() as u64,
    }
}

fn value_truncation(value: &OwnedValue) -> Truncation {
    match value.as_ref() {
        ValueRef::Temporal { truncation, .. }
        | ValueRef::Text { truncation, .. }
        | ValueRef::Structured { truncation, .. }
        | ValueRef::Binary { truncation, .. }
        | ValueRef::Invalid { truncation, .. }
        | ValueRef::Unknown { truncation, .. } => truncation,
        _ => Truncation::Complete,
    }
}

fn append_value(value: &OwnedValue, arena: &mut Vec<u8>) {
    match value.as_ref() {
        ValueRef::Null => {}
        ValueRef::Boolean(value) => arena.push(u8::from(value)),
        ValueRef::Signed(value) => arena.extend_from_slice(&value.to_be_bytes()),
        ValueRef::Unsigned(value) | ValueRef::Float64Bits(value) => {
            arena.extend_from_slice(&value.to_be_bytes());
        }
        ValueRef::Decimal(value)
        | ValueRef::Temporal { value, .. }
        | ValueRef::Text { value, .. }
        | ValueRef::Structured { value, .. } => {
            arena.extend_from_slice(value.as_bytes());
        }
        ValueRef::Binary { value, .. }
        | ValueRef::Invalid { payload: value, .. }
        | ValueRef::Unknown { payload: value, .. } => arena.extend_from_slice(value),
    }
}

impl fmt::Debug for ResultPage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResultPage")
            .field("envelope", &self.envelope)
            .field("buffers", &self.buffers)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CellRef<'a> {
    kind: ValueKind,
    is_null: bool,
    truncation: Truncation,
    bytes: &'a [u8],
}

impl<'a> CellRef<'a> {
    #[must_use]
    pub const fn kind(self) -> ValueKind {
        self.kind
    }

    #[must_use]
    pub const fn is_null(self) -> bool {
        self.is_null
    }

    #[must_use]
    pub const fn truncation(self) -> Truncation {
        self.truncation
    }

    #[must_use]
    pub const fn bytes(self) -> &'a [u8] {
        self.bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageAccessError {
    RowOutsidePage,
    ColumnOutsidePage,
}

impl fmt::Display for PageAccessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::RowOutsidePage => "row is outside the page",
            Self::ColumnOutsidePage => "column is outside the page",
        })
    }
}

impl Error for PageAccessError {}

fn validate_parts(
    envelope: PageEnvelope,
    buffers: &PageBuffers,
) -> Result<(), PageValidationError> {
    let PageBuffers {
        columns,
        cell_offsets: offsets,
        null_bitmap,
        value_kinds: kinds,
        truncations,
        arena,
    } = buffers;
    let cells = envelope.row_count as u64 * envelope.column_count as u64;
    check_len(
        envelope.column_count as u64,
        columns.len(),
        |expected, actual| PageValidationError::ColumnCountMismatch {
            expected: expected as u32,
            actual,
        },
    )?;
    for (index, column) in columns.iter().enumerate() {
        if column.engine_type.engine() != envelope.engine {
            return Err(PageValidationError::ColumnEngineMismatch {
                column: index as u32,
            });
        }
    }
    let column_text_bytes = columns.iter().try_fold(0_u64, |total, column| {
        total.checked_add(column.column_text_byte_len())
    });
    let Some(column_text_bytes) = column_text_bytes else {
        return Err(PageValidationError::ColumnTextLengthOverflow);
    };
    if column_text_bytes != envelope.column_text_byte_len {
        return Err(PageValidationError::ColumnTextLengthMismatch {
            declared: envelope.column_text_byte_len,
            actual: column_text_bytes,
        });
    }
    if arena.len() as u64 != envelope.arena_byte_len {
        return Err(PageValidationError::ArenaLengthMismatch {
            declared: envelope.arena_byte_len,
            actual: arena.len() as u64,
        });
    }
    check_len(cells + 1, offsets.len(), |expected, actual| {
        PageValidationError::CellOffsetCountMismatch { expected, actual }
    })?;
    check_len(cells, kinds.len(), |expected, actual| {
        PageValidationError::KindCountMismatch { expected, actual }
    })?;
    check_len(cells, truncations.len(), |expected, actual| {
        PageValidationError::TruncationCountMismatch { expected, actual }
    })?;
    let bitmap_bytes = cells.div_ceil(8);
    check_len(bitmap_bytes, null_bitmap.len(), |expected, actual| {
        PageValidationError::NullBitmapLengthMismatch { expected, actual }
    })?;
    if !cells.is_multiple_of(8) && null_bitmap.last().copied().unwrap_or(0) >> (cells % 8) != 0 {
        return Err(PageValidationError::NonzeroNullPadding);
    }
    if offsets[0] != 0 {
        return Err(PageValidationError::FirstOffsetNotZero { actual: offsets[0] });
    }
    for cell in 0..cells as usize {
        let start = offsets[cell];
        let end = offsets[cell + 1];
        if end < start {
            return Err(PageValidationError::OffsetDecreases { cell: cell as u64 });
        }
        if end > envelope.arena_byte_len {
            return Err(PageValidationError::OffsetOutsideArena {
                cell: cell as u64,
                offset: end,
                arena_bytes: envelope.arena_byte_len,
            });
        }
        let is_null = null_bit(null_bitmap, cell);
        if is_null != (kinds[cell] == ValueKind::Null) {
            return Err(PageValidationError::NullKindMismatch { cell: cell as u64 });
        }
        if is_null && start != end {
            return Err(PageValidationError::NullHasBytes { cell: cell as u64 });
        }
        if is_null && truncations[cell] != Truncation::Complete {
            return Err(PageValidationError::NullIsTruncated { cell: cell as u64 });
        }
        let column = cell / envelope.row_count as usize;
        if is_null && !columns[column].nullable {
            return Err(PageValidationError::NullInNonNullableColumn {
                cell: cell as u64,
                column: column as u32,
            });
        }
        let bytes = &arena[start as usize..end as usize];
        validate_encoding(cell as u64, kinds[cell], bytes)?;
        if truncations[cell] != Truncation::Complete
            && !matches!(
                kinds[cell],
                ValueKind::Temporal
                    | ValueKind::Text
                    | ValueKind::Structured
                    | ValueKind::Binary
                    | ValueKind::Invalid
                    | ValueKind::Unknown
            )
        {
            return Err(PageValidationError::UnsupportedTruncationKind {
                cell: cell as u64,
                kind: kinds[cell],
            });
        }
        if let Truncation::Truncated {
            original_byte_len: Some(original),
        } = truncations[cell]
        {
            let stored = end - start;
            if original <= stored {
                return Err(PageValidationError::InvalidTruncationLength {
                    cell: cell as u64,
                    stored,
                    original,
                });
            }
        }
    }
    if offsets[cells as usize] != envelope.arena_byte_len {
        return Err(PageValidationError::FinalOffsetMismatch {
            offset: offsets[cells as usize],
            arena_bytes: envelope.arena_byte_len,
        });
    }
    Ok(())
}

fn validate_encoding(cell: u64, kind: ValueKind, bytes: &[u8]) -> Result<(), PageValidationError> {
    match kind {
        ValueKind::Null if bytes.is_empty() => Ok(()),
        ValueKind::Null => Err(PageValidationError::NullHasBytes { cell }),
        ValueKind::Boolean if matches!(bytes, [0] | [1]) => Ok(()),
        ValueKind::Boolean => Err(PageValidationError::InvalidBooleanEncoding { cell }),
        ValueKind::Signed | ValueKind::Unsigned | ValueKind::Float64 if bytes.len() == 8 => Ok(()),
        ValueKind::Signed | ValueKind::Unsigned | ValueKind::Float64 => {
            Err(PageValidationError::InvalidFixedWidthEncoding {
                cell,
                expected: 8,
                actual: bytes.len() as u64,
            })
        }
        ValueKind::Decimal | ValueKind::Temporal | ValueKind::Text | ValueKind::Structured
            if std::str::from_utf8(bytes).is_ok() =>
        {
            Ok(())
        }
        ValueKind::Decimal | ValueKind::Temporal | ValueKind::Text | ValueKind::Structured => {
            Err(PageValidationError::InvalidUtf8Encoding { cell })
        }
        ValueKind::Binary | ValueKind::Invalid | ValueKind::Unknown => Ok(()),
    }
}

fn check_len(
    expected: u64,
    actual: usize,
    error: impl FnOnce(u64, u64) -> PageValidationError,
) -> Result<(), PageValidationError> {
    let actual = actual as u64;
    if expected == actual {
        Ok(())
    } else {
        Err(error(expected, actual))
    }
}

fn null_bit(bitmap: &[u8], cell: usize) -> bool {
    bitmap[cell / 8] & (1 << (cell % 8)) != 0
}

const PAGE_V1_MAGIC: &[u8; 4] = b"TRP1";

struct ByteCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ByteCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    const fn remaining(self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    fn take(&mut self, len: usize) -> Option<&'a [u8]> {
        let end = self.offset.checked_add(len)?;
        let slice = self.bytes.get(self.offset..end)?;
        self.offset = end;
        Some(slice)
    }

    fn u8(&mut self) -> Option<u8> {
        let bytes = self.take(1)?;
        Some(bytes[0])
    }

    fn u16_le(&mut self) -> Option<u16> {
        let bytes = self.take(2)?;
        Some(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn u32_le(&mut self) -> Option<u32> {
        let bytes = self.take(4)?;
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn u64_le(&mut self) -> Option<u64> {
        let bytes = self.take(8)?;
        Some(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn array16(&mut self) -> Option<[u8; 16]> {
        let bytes = self.take(16)?;
        let mut out = [0_u8; 16];
        out.copy_from_slice(bytes);
        Some(out)
    }
}

fn estimate_encoded_len(envelope: PageEnvelope, buffers: &PageBuffers) -> usize {
    let cells = envelope.row_count as usize * envelope.column_count as usize;
    let mut len: usize = 4 + 2 + 16 + 8 + 1 + 8 + 4 + 4 + 1 + 8 + 8 + 8 + 1 + 2;
    for column in &buffers.columns {
        len = len
            .saturating_add(4)
            .saturating_add(column.name().len())
            .saturating_add(1)
            .saturating_add(4)
            .saturating_add(column.engine_type().name().len())
            .saturating_add(1);
    }
    len = len
        .saturating_add(cells.saturating_add(1).saturating_mul(8))
        .saturating_add(cells.div_ceil(8))
        .saturating_add(cells)
        .saturating_add(cells.saturating_mul(1 + 8))
        .saturating_add(buffers.arena.len());
    len
}

fn write_bounded_str(out: &mut Vec<u8>, value: &str) {
    let len = u32::try_from(value.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn read_length_prefixed<'a>(
    cursor: &mut ByteCursor<'a>,
) -> Result<&'a [u8], PageValidationError> {
    let len = cursor
        .u32_le()
        .ok_or(PageValidationError::TruncatedEncoding)? as usize;
    cursor
        .take(len)
        .ok_or(PageValidationError::TruncatedEncoding)
}

const fn engine_to_wire(engine: Engine) -> u8 {
    match engine {
        Engine::PostgreSql => 0,
        Engine::ClickHouse => 1,
        Engine::Redis => 2,
    }
}

const fn engine_from_wire(tag: u8) -> Result<Engine, PageValidationError> {
    match tag {
        0 => Ok(Engine::PostgreSql),
        1 => Ok(Engine::ClickHouse),
        2 => Ok(Engine::Redis),
        _ => Err(PageValidationError::InvalidEngineTag),
    }
}

const fn value_kind_to_wire(kind: ValueKind) -> u8 {
    match kind {
        ValueKind::Null => 0,
        ValueKind::Boolean => 1,
        ValueKind::Signed => 2,
        ValueKind::Unsigned => 3,
        ValueKind::Float64 => 4,
        ValueKind::Decimal => 5,
        ValueKind::Temporal => 6,
        ValueKind::Text => 7,
        ValueKind::Structured => 8,
        ValueKind::Binary => 9,
        ValueKind::Invalid => 10,
        ValueKind::Unknown => 11,
    }
}

const fn value_kind_from_wire(tag: u8) -> Result<ValueKind, PageValidationError> {
    match tag {
        0 => Ok(ValueKind::Null),
        1 => Ok(ValueKind::Boolean),
        2 => Ok(ValueKind::Signed),
        3 => Ok(ValueKind::Unsigned),
        4 => Ok(ValueKind::Float64),
        5 => Ok(ValueKind::Decimal),
        6 => Ok(ValueKind::Temporal),
        7 => Ok(ValueKind::Text),
        8 => Ok(ValueKind::Structured),
        9 => Ok(ValueKind::Binary),
        10 => Ok(ValueKind::Invalid),
        11 => Ok(ValueKind::Unknown),
        _ => Err(PageValidationError::InvalidValueKindTag),
    }
}

fn write_truncation(out: &mut Vec<u8>, truncation: Truncation) {
    match truncation {
        Truncation::Complete => out.push(0),
        Truncation::Truncated {
            original_byte_len: None,
        } => out.push(1),
        Truncation::Truncated {
            original_byte_len: Some(len),
        } => {
            out.push(2);
            out.extend_from_slice(&len.to_le_bytes());
        }
    }
}

fn read_truncation(cursor: &mut ByteCursor<'_>) -> Result<Truncation, PageValidationError> {
    match cursor.u8().ok_or(PageValidationError::TruncatedEncoding)? {
        0 => Ok(Truncation::Complete),
        1 => Ok(Truncation::Truncated {
            original_byte_len: None,
        }),
        2 => {
            let len = cursor
                .u64_le()
                .ok_or(PageValidationError::TruncatedEncoding)?;
            Ok(Truncation::Truncated {
                original_byte_len: Some(len),
            })
        }
        _ => Err(PageValidationError::InvalidTruncationTag),
    }
}
