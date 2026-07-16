use std::{error::Error, fmt};

use bytes::Bytes;
use clickhouse::{Client, Compression, query::BytesCursor};
use tablerock_core::{
    BoundedBytes, BoundedText, ByteLimit, ColumnMetadata, Engine, EngineType, OwnedValue,
    PageDelivery, PageFacts, PageIdentity, PageLimits, PageValidationError, PageWarning,
    PageWarnings, ResultPage, RowTotal, Truncation, ValueKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClickHouseTlsMode {
    Disable,
    Require,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClickHouseCompression {
    None,
    Lz4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClickHouseProbeQuery {
    TypedValues,
}

impl ClickHouseProbeQuery {
    const fn sql(self) -> &'static str {
        match self {
            Self::TypedValues => {
                "SELECT number AS id, toInt64(-7) AS signed_value, \
                 toFloat64(if(number = 1, -0.0, 1.5)) AS float_value, \
                 concat('row-', toString(number)) AS label, \
                 CAST(if(number = 1, NULL, 'present'), 'Nullable(String)') AS optional, \
                 CAST(unhex('00FF'), 'FixedString(2)') AS binary_value \
                 FROM numbers(3)"
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ClickHouseConnectConfig {
    host: BoundedText,
    port: u16,
    database: BoundedText,
    user: BoundedText,
    tls: ClickHouseTlsMode,
    compression: ClickHouseCompression,
}

impl ClickHouseConnectConfig {
    #[must_use]
    pub const fn new(
        host: BoundedText,
        port: u16,
        database: BoundedText,
        user: BoundedText,
        tls: ClickHouseTlsMode,
        compression: ClickHouseCompression,
    ) -> Self {
        Self {
            host,
            port,
            database,
            user,
            tls,
            compression,
        }
    }
}

impl fmt::Debug for ClickHouseConnectConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClickHouseConnectConfig")
            .field("host_bytes", &self.host.len())
            .field("port", &self.port)
            .field("database_bytes", &self.database.len())
            .field("user_bytes", &self.user.len())
            .field("tls", &self.tls)
            .field("compression", &self.compression)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClickHouseError {
    Query,
    Protocol,
    UnsupportedType,
    InvalidLimits,
    Page(PageValidationError),
}

impl fmt::Display for ClickHouseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Query => "ClickHouse query failed",
            Self::Protocol => "ClickHouse returned an invalid result stream",
            Self::UnsupportedType => "ClickHouse returned a type not decoded by this checkpoint",
            Self::InvalidLimits => "ClickHouse stream limits are invalid",
            Self::Page(_) => "ClickHouse result page failed validation",
        })
    }
}

impl Error for ClickHouseError {}

#[derive(Clone)]
pub struct ClickHouseSession {
    client: Client,
}

impl ClickHouseSession {
    #[must_use]
    pub fn connect(config: &ClickHouseConnectConfig) -> Self {
        let scheme = match config.tls {
            ClickHouseTlsMode::Disable => "http",
            ClickHouseTlsMode::Require => "https",
        };
        let compression = match config.compression {
            ClickHouseCompression::None => Compression::None,
            ClickHouseCompression::Lz4 => Compression::Lz4,
        };
        let client = Client::default()
            .with_url(format!(
                "{scheme}://{}:{}",
                config.host.as_str(),
                config.port
            ))
            .with_database(config.database.as_str())
            .with_user(config.user.as_str())
            .with_compression(compression)
            .with_product_info("tablerock", env!("CARGO_PKG_VERSION"));
        Self { client }
    }

    pub async fn stream_probe(
        &self,
        query: ClickHouseProbeQuery,
        query_id: &BoundedText,
        limits: PageLimits,
        max_cell_bytes: u64,
    ) -> Result<ClickHouseRowStream, ClickHouseError> {
        if limits.max_rows() == 0
            || limits.max_columns() == 0
            || limits.max_arena_bytes() == 0
            || limits.max_column_text_bytes() == 0
            || max_cell_bytes == 0
            || query_id.is_empty()
        {
            return Err(ClickHouseError::InvalidLimits);
        }
        let cursor = self
            .client
            .query(query.sql())
            .with_setting("query_id", query_id.as_str())
            .fetch_bytes("RowBinaryWithNamesAndTypes")
            .map_err(|_| ClickHouseError::Query)?;
        ClickHouseRowStream::start(cursor, limits, max_cell_bytes).await
    }
}

pub struct ClickHouseRowStream {
    reader: ChunkReader,
    columns: Vec<ColumnMetadata>,
    types: Vec<ClickHouseType>,
    limits: PageLimits,
    max_cell_bytes: u64,
    complete: bool,
}

impl ClickHouseRowStream {
    async fn start(
        cursor: BytesCursor,
        limits: PageLimits,
        max_cell_bytes: u64,
    ) -> Result<Self, ClickHouseError> {
        let mut reader = ChunkReader::new(cursor);
        let count = read_leb128(&mut reader).await?;
        let count = usize::try_from(count).map_err(|_| ClickHouseError::Protocol)?;
        if count == 0 || count > limits.max_columns() as usize {
            return Err(ClickHouseError::Protocol);
        }
        let mut names = Vec::with_capacity(count);
        for _ in 0..count {
            names.push(read_metadata_string(&mut reader, limits.max_column_text_bytes()).await?);
        }
        let mut types = Vec::with_capacity(count);
        let mut columns = Vec::with_capacity(count);
        for name in names {
            let raw = read_metadata_string(&mut reader, limits.max_column_text_bytes()).await?;
            let type_ = ClickHouseType::parse(&raw)?;
            let name =
                BoundedText::copy_from_str(&name, ByteLimit::new(limits.max_column_text_bytes()))
                    .map_err(|_| ClickHouseError::Protocol)?;
            let engine_type = EngineType::new(
                Engine::ClickHouse,
                BoundedText::copy_from_str(&raw, ByteLimit::new(limits.max_column_text_bytes()))
                    .map_err(|_| ClickHouseError::Protocol)?,
            )
            .map_err(|_| ClickHouseError::Protocol)?;
            columns.push(ColumnMetadata::new(name, engine_type, type_.nullable()));
            types.push(type_);
        }
        Ok(Self {
            reader,
            columns,
            types,
            limits,
            max_cell_bytes,
            complete: false,
        })
    }

    pub async fn next_page(
        &mut self,
        identity: PageIdentity,
        start_row: u64,
    ) -> Result<Option<ResultPage>, ClickHouseError> {
        if self.complete || !self.reader.has_data().await? {
            self.complete = true;
            return Ok(None);
        }
        let mut values = Vec::new();
        let mut rows = 0_u32;
        let mut arena_remaining = self.limits.max_arena_bytes();
        while rows < self.limits.max_rows() {
            for type_ in &self.types {
                let value = type_
                    .read(&mut self.reader, self.max_cell_bytes.min(arena_remaining))
                    .await?;
                arena_remaining = arena_remaining.saturating_sub(encoded_len(&value));
                values.push(value);
            }
            rows += 1;
            if !self.reader.has_data().await? {
                self.complete = true;
                break;
            }
        }
        let delivery = if self.complete {
            PageDelivery::Final
        } else {
            PageDelivery::Partial
        };
        let mut warnings = PageWarnings::none();
        if delivery == PageDelivery::Partial {
            warnings = warnings.with(PageWarning::RowLimitReached);
        }
        if values.iter().any(OwnedValue::is_truncated) {
            warnings = warnings.with(PageWarning::ByteLimitReached);
        }
        if values
            .iter()
            .any(|value| value.kind() == ValueKind::Unknown)
        {
            warnings = warnings.with(PageWarning::UnknownValues);
        }
        ResultPage::from_row_major(
            identity,
            start_row,
            RowTotal::Unknown,
            PageFacts::new(delivery, warnings),
            self.columns.clone(),
            values,
            self.limits,
        )
        .map(Some)
        .map_err(ClickHouseError::Page)
    }
}

#[derive(Debug, Clone)]
enum ClickHouseType {
    UInt64,
    Int64,
    Float64,
    String,
    FixedString(usize),
    Nullable(Box<Self>),
}

impl ClickHouseType {
    fn parse(raw: &str) -> Result<Self, ClickHouseError> {
        if let Some(inner) = raw
            .strip_prefix("Nullable(")
            .and_then(|raw| raw.strip_suffix(')'))
        {
            return Ok(Self::Nullable(Box::new(Self::parse(inner)?)));
        }
        if let Some(length) = raw
            .strip_prefix("FixedString(")
            .and_then(|raw| raw.strip_suffix(')'))
        {
            let length = length.parse().map_err(|_| ClickHouseError::Protocol)?;
            return Ok(Self::FixedString(length));
        }
        match raw {
            "UInt64" => Ok(Self::UInt64),
            "Int64" => Ok(Self::Int64),
            "Float64" => Ok(Self::Float64),
            "String" => Ok(Self::String),
            _ => Err(ClickHouseError::UnsupportedType),
        }
    }

    const fn nullable(&self) -> bool {
        matches!(self, Self::Nullable(_))
    }

    fn read<'a>(
        &'a self,
        reader: &'a mut ChunkReader,
        limit: u64,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<OwnedValue, ClickHouseError>> + Send + 'a>,
    > {
        Box::pin(async move {
            match self {
                Self::Nullable(inner) => match reader.read_u8().await? {
                    0 => inner.read(reader, limit).await,
                    1 => Ok(OwnedValue::null()),
                    _ => Err(ClickHouseError::Protocol),
                },
                Self::UInt64 => {
                    fixed_value(reader, limit, |bytes| {
                        OwnedValue::unsigned(u64::from_le_bytes(bytes))
                    })
                    .await
                }
                Self::Int64 => {
                    fixed_value(reader, limit, |bytes| {
                        OwnedValue::signed(i64::from_le_bytes(bytes))
                    })
                    .await
                }
                Self::Float64 => {
                    fixed_value(reader, limit, |bytes| {
                        OwnedValue::float64_bits(u64::from_le_bytes(bytes))
                    })
                    .await
                }
                Self::String => {
                    let original = read_leb128(reader).await?;
                    let (bytes, truncation) = read_bounded(reader, original, limit).await?;
                    match std::str::from_utf8(&bytes) {
                        Ok(text) => OwnedValue::text(
                            BoundedText::copy_from_str(text, ByteLimit::new(limit))
                                .map_err(|_| ClickHouseError::Protocol)?,
                            truncation,
                        )
                        .map_err(|_| ClickHouseError::Protocol),
                        Err(_) => OwnedValue::binary(
                            BoundedBytes::from_vec(bytes, ByteLimit::new(limit))
                                .map_err(|_| ClickHouseError::Protocol)?,
                            truncation,
                        )
                        .map_err(|_| ClickHouseError::Protocol),
                    }
                }
                Self::FixedString(length) => {
                    let original = u64::try_from(*length).map_err(|_| ClickHouseError::Protocol)?;
                    let (bytes, truncation) = read_bounded(reader, original, limit).await?;
                    OwnedValue::binary(
                        BoundedBytes::from_vec(bytes, ByteLimit::new(limit))
                            .map_err(|_| ClickHouseError::Protocol)?,
                        truncation,
                    )
                    .map_err(|_| ClickHouseError::Protocol)
                }
            }
        })
    }
}

async fn fixed_value(
    reader: &mut ChunkReader,
    limit: u64,
    decode: impl FnOnce([u8; 8]) -> OwnedValue,
) -> Result<OwnedValue, ClickHouseError> {
    let mut bytes = [0; 8];
    reader.read_exact(&mut bytes).await?;
    if limit >= 8 {
        Ok(decode(bytes))
    } else {
        let stored_len = usize::try_from(limit)
            .unwrap_or(usize::MAX)
            .min(bytes.len());
        let payload = BoundedBytes::copy_from_slice(&bytes[..stored_len], ByteLimit::new(limit))
            .map_err(|_| ClickHouseError::Protocol)?;
        let engine_type = EngineType::new(
            Engine::ClickHouse,
            BoundedText::copy_from_str("FixedWidth", ByteLimit::new(10))
                .map_err(|_| ClickHouseError::Protocol)?,
        )
        .map_err(|_| ClickHouseError::Protocol)?;
        OwnedValue::unknown(
            engine_type,
            payload,
            Truncation::Truncated {
                original_byte_len: Some(8),
            },
        )
        .map_err(|_| ClickHouseError::Protocol)
    }
}

async fn read_metadata_string(
    reader: &mut ChunkReader,
    limit: u64,
) -> Result<String, ClickHouseError> {
    let length = read_leb128(reader).await?;
    if length > limit {
        return Err(ClickHouseError::Protocol);
    }
    let mut bytes = vec![0; usize::try_from(length).map_err(|_| ClickHouseError::Protocol)?];
    reader.read_exact(&mut bytes).await?;
    String::from_utf8(bytes).map_err(|_| ClickHouseError::Protocol)
}

async fn read_leb128(reader: &mut ChunkReader) -> Result<u64, ClickHouseError> {
    let mut value = 0_u64;
    for shift in (0..=63).step_by(7) {
        let byte = reader.read_u8().await?;
        let payload = u64::from(byte & 0x7f);
        if shift == 63 && payload > 1 {
            return Err(ClickHouseError::Protocol);
        }
        value |= payload << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(ClickHouseError::Protocol)
}

async fn read_bounded(
    reader: &mut ChunkReader,
    original: u64,
    limit: u64,
) -> Result<(Vec<u8>, Truncation), ClickHouseError> {
    let stored_len = original.min(limit);
    let mut stored = vec![0; usize::try_from(stored_len).map_err(|_| ClickHouseError::Protocol)?];
    reader.read_exact(&mut stored).await?;
    let mut remaining = original - stored_len;
    let mut discard = [0; 8192];
    while remaining != 0 {
        let take = remaining.min(discard.len() as u64) as usize;
        reader.read_exact(&mut discard[..take]).await?;
        remaining -= take as u64;
    }
    let truncation = if stored_len == original {
        Truncation::Complete
    } else {
        Truncation::Truncated {
            original_byte_len: Some(original),
        }
    };
    Ok((stored, truncation))
}

struct ChunkReader {
    cursor: BytesCursor,
    chunk: Bytes,
}

impl ChunkReader {
    fn new(cursor: BytesCursor) -> Self {
        Self {
            cursor,
            chunk: Bytes::new(),
        }
    }

    async fn has_data(&mut self) -> Result<bool, ClickHouseError> {
        while self.chunk.is_empty() {
            let Some(chunk) = self
                .cursor
                .next()
                .await
                .map_err(|_| ClickHouseError::Query)?
            else {
                return Ok(false);
            };
            self.chunk = chunk;
        }
        Ok(true)
    }

    async fn read_u8(&mut self) -> Result<u8, ClickHouseError> {
        if !self.has_data().await? {
            return Err(ClickHouseError::Protocol);
        }
        Ok(self.chunk.split_to(1)[0])
    }

    async fn read_exact(&mut self, output: &mut [u8]) -> Result<(), ClickHouseError> {
        let mut offset = 0;
        while offset < output.len() {
            if !self.has_data().await? {
                return Err(ClickHouseError::Protocol);
            }
            let take = (output.len() - offset).min(self.chunk.len());
            output[offset..offset + take].copy_from_slice(&self.chunk.split_to(take));
            offset += take;
        }
        Ok(())
    }
}

fn encoded_len(value: &OwnedValue) -> u64 {
    match value.as_ref() {
        tablerock_core::ValueRef::Null => 0,
        tablerock_core::ValueRef::Boolean(_) => 1,
        tablerock_core::ValueRef::Signed(_)
        | tablerock_core::ValueRef::Unsigned(_)
        | tablerock_core::ValueRef::Float64Bits(_) => 8,
        tablerock_core::ValueRef::Decimal(value) | tablerock_core::ValueRef::Text { value, .. } => {
            value.len() as u64
        }
        tablerock_core::ValueRef::Binary { value, .. }
        | tablerock_core::ValueRef::Invalid { payload: value, .. }
        | tablerock_core::ValueRef::Unknown { payload: value, .. } => value.len() as u64,
    }
}
