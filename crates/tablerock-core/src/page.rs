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
}

impl PageWarning {
    const COUNT: usize = 5;

    const fn index(self) -> usize {
        match self {
            Self::RowLimitReached => 0,
            Self::ByteLimitReached => 1,
            Self::UnknownValues => 2,
            Self::InvalidValues => 3,
            Self::PartialFailure => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PageWarnings(u16);

const _: () = assert!(PageWarning::COUNT <= u16::BITS as usize);

impl PageWarnings {
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
}

fn value_byte_len(value: &OwnedValue) -> u64 {
    match value.as_ref() {
        ValueRef::Null => 0,
        ValueRef::Boolean(_) => 1,
        ValueRef::Signed(_) | ValueRef::Unsigned(_) | ValueRef::Float64Bits(_) => 8,
        ValueRef::Decimal(value) | ValueRef::Text { value, .. } => value.len() as u64,
        ValueRef::Binary { value, .. }
        | ValueRef::Invalid { payload: value, .. }
        | ValueRef::Unknown { payload: value, .. } => value.len() as u64,
    }
}

fn value_truncation(value: &OwnedValue) -> Truncation {
    match value.as_ref() {
        ValueRef::Text { truncation, .. }
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
        ValueRef::Decimal(value) | ValueRef::Text { value, .. } => {
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
                ValueKind::Text | ValueKind::Binary | ValueKind::Invalid | ValueKind::Unknown
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
        ValueKind::Decimal | ValueKind::Text if std::str::from_utf8(bytes).is_ok() => Ok(()),
        ValueKind::Decimal | ValueKind::Text => {
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
