use std::{error::Error, fmt};

use bytes::Bytes;
use clickhouse::{Client, Compression, query::BytesCursor};
use tablerock_core::{
    BoundedBytes, BoundedText, ByteLimit, ColumnMetadata, Engine, EngineType, OwnedValue,
    PageDelivery, PageFacts, PageIdentity, PageLimits, PageValidationError, PageWarning,
    PageWarnings, ResultPage, RowTotal, Truncation, ValueKind,
};

const MAX_CLICKHOUSE_TYPE_DEPTH: u8 = 64;
const MAX_STRUCTURED_NODES: u64 = 1_000_000;

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
    ComplexScalars,
    StructuredValues,
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
            Self::ComplexScalars => {
                "SELECT CAST(1, 'Bool') AS boolean_value, toUInt8(255) AS uint8_value, \
                 toUInt16(65535) AS uint16_value, toUInt32(4294967295) AS uint32_value, \
                 toUInt128('340282366920938463463374607431768211455') AS uint128_value, \
                 toInt128('-170141183460469231731687303715884105728') AS int128_value, \
                 toUInt256('115792089237316195423570985008687907853269984665640564039457584007913129639935') AS uint256_value, \
                 toInt256('-57896044618658097711785492504343953926634992332820282019728792003956564819968') AS int256_value, \
                 CAST('12345678901234567890123456789.123456789', 'Decimal256(9)') AS decimal_value, \
                 toFloat32(1.5) AS float32_value, toDate('2024-02-29') AS date_value, \
                 toDate32('1900-01-01') AS date32_value, \
                 toDateTime('2024-02-29 12:34:56', 'UTC') AS datetime_value, \
                 toDateTime64('2024-02-29 12:34:56.123456789', 9, 'UTC') AS datetime64_value, \
                 toUUID('550e8400-e29b-41d4-a716-446655440000') AS uuid_value, \
                 toIPv4('192.0.2.1') AS ipv4_value, toIPv6('2001:db8::1') AS ipv6_value, \
                 CAST(7, 'Enum8(\\'ready\\' = 7, \\'done\\' = 9)') AS enum_value, \
                 CAST('wrapped', 'LowCardinality(String)') AS low_cardinality_value, \
                 toInt8(-128) AS int8_value, toInt16(-32768) AS int16_value, \
                 toInt32(-2147483648) AS int32_value"
            }
            Self::StructuredValues => {
                "SELECT [toUInt8(1), 2, 3] AS array_value, \
                 tuple(toInt64(-7), 'quoted\\n', CAST(NULL, 'Nullable(UInt8)')) AS tuple_value, \
                 map('a', toUInt16(1), 'b', toUInt16(2)) AS map_value, \
                 CAST([(1, 'one'), (2, 'two')], 'Array(Tuple(id UInt8, label String))') AS nested_value, \
                 [CAST(unhex('00FF'), 'FixedString(2)')] AS binary_array"
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
                arena_remaining = arena_remaining.saturating_sub(value.encoded_byte_len());
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
    Boolean,
    Unsigned(usize),
    Signed(usize),
    BigInteger {
        bytes: usize,
        signed: bool,
        type_name: String,
    },
    Decimal {
        bytes: usize,
        scale: u32,
        type_name: String,
    },
    Float32,
    Float64,
    String,
    Binary(usize),
    Nullable(Box<Self>),
    Array(Box<Self>),
    Tuple(Vec<Self>),
    Map(Box<Self>, Box<Self>),
}

impl ClickHouseType {
    fn parse(raw: &str) -> Result<Self, ClickHouseError> {
        Self::parse_at(raw, 0)
    }

    fn parse_at(raw: &str, depth: u8) -> Result<Self, ClickHouseError> {
        if depth > MAX_CLICKHOUSE_TYPE_DEPTH {
            return Err(ClickHouseError::Protocol);
        }
        if let Some(inner) = raw
            .strip_prefix("Nullable(")
            .and_then(|raw| raw.strip_suffix(')'))
        {
            return Ok(Self::Nullable(Box::new(Self::parse_at(inner, depth + 1)?)));
        }
        if let Some(length) = raw
            .strip_prefix("FixedString(")
            .and_then(|raw| raw.strip_suffix(')'))
        {
            let length = length.parse().map_err(|_| ClickHouseError::Protocol)?;
            return Ok(Self::Binary(length));
        }
        if let Some(inner) = call_argument(raw, "LowCardinality") {
            return Self::parse_at(inner, depth + 1);
        }
        if let Some(inner) = call_argument(raw, "Array") {
            return Ok(Self::Array(Box::new(Self::parse_at(inner, depth + 1)?)));
        }
        if let Some(inner) = call_argument(raw, "Tuple") {
            let fields = split_type_arguments(inner)?
                .into_iter()
                .map(|field| Self::parse_named(field, depth + 1))
                .collect::<Result<Vec<_>, _>>()?;
            if fields.is_empty() {
                return Err(ClickHouseError::Protocol);
            }
            return Ok(Self::Tuple(fields));
        }
        if let Some(inner) = call_argument(raw, "Map") {
            let fields = split_type_arguments(inner)?;
            if fields.len() != 2 {
                return Err(ClickHouseError::Protocol);
            }
            return Ok(Self::Map(
                Box::new(Self::parse_at(fields[0], depth + 1)?),
                Box::new(Self::parse_at(fields[1], depth + 1)?),
            ));
        }
        if let Some(inner) = call_argument(raw, "Nested") {
            let fields = split_type_arguments(inner)?
                .into_iter()
                .map(|field| Self::parse_named(field, depth + 1))
                .collect::<Result<Vec<_>, _>>()?;
            if fields.is_empty() {
                return Err(ClickHouseError::Protocol);
            }
            return Ok(Self::Array(Box::new(Self::Tuple(fields))));
        }
        if raw.starts_with("Enum8(") {
            return Ok(Self::Signed(1));
        }
        if raw.starts_with("Enum16(") {
            return Ok(Self::Signed(2));
        }
        if let Some(scale) = decimal_scale(raw, "Decimal32")? {
            return Ok(Self::decimal(4, scale, raw));
        }
        if let Some(scale) = decimal_scale(raw, "Decimal64")? {
            return Ok(Self::decimal(8, scale, raw));
        }
        if let Some(scale) = decimal_scale(raw, "Decimal128")? {
            return Ok(Self::decimal(16, scale, raw));
        }
        if let Some(scale) = decimal_scale(raw, "Decimal256")? {
            return Ok(Self::decimal(32, scale, raw));
        }
        if let Some(inner) = call_argument(raw, "Decimal") {
            let (precision, scale) = split_pair(inner)?;
            let precision: u32 = precision.parse().map_err(|_| ClickHouseError::Protocol)?;
            let scale: u32 = scale.parse().map_err(|_| ClickHouseError::Protocol)?;
            let bytes = match precision {
                1..=9 => 4,
                10..=18 => 8,
                19..=38 => 16,
                39..=76 => 32,
                _ => return Err(ClickHouseError::Protocol),
            };
            return Ok(Self::decimal(bytes, scale, raw));
        }
        if call_argument(raw, "DateTime").is_some() {
            return Ok(Self::Unsigned(4));
        }
        if call_argument(raw, "DateTime64").is_some() {
            return Ok(Self::Signed(8));
        }
        match raw {
            "Bool" => Ok(Self::Boolean),
            "UInt8" => Ok(Self::Unsigned(1)),
            "UInt16" | "Date" => Ok(Self::Unsigned(2)),
            "UInt32" | "DateTime" => Ok(Self::Unsigned(4)),
            "UInt64" => Ok(Self::Unsigned(8)),
            "Int8" => Ok(Self::Signed(1)),
            "Int16" => Ok(Self::Signed(2)),
            "Int32" | "Date32" => Ok(Self::Signed(4)),
            "Int64" | "DateTime64" => Ok(Self::Signed(8)),
            "UInt128" => Ok(Self::big_integer(16, false, raw)),
            "UInt256" => Ok(Self::big_integer(32, false, raw)),
            "Int128" => Ok(Self::big_integer(16, true, raw)),
            "Int256" => Ok(Self::big_integer(32, true, raw)),
            "Float32" => Ok(Self::Float32),
            "Float64" => Ok(Self::Float64),
            "String" => Ok(Self::String),
            "IPv4" => Ok(Self::Binary(4)),
            "UUID" | "IPv6" => Ok(Self::Binary(16)),
            _ => Err(ClickHouseError::UnsupportedType),
        }
    }

    fn decimal(bytes: usize, scale: u32, type_name: &str) -> Self {
        Self::Decimal {
            bytes,
            scale,
            type_name: type_name.to_owned(),
        }
    }

    fn big_integer(bytes: usize, signed: bool, type_name: &str) -> Self {
        Self::BigInteger {
            bytes,
            signed,
            type_name: type_name.to_owned(),
        }
    }

    fn parse_named(raw: &str, depth: u8) -> Result<Self, ClickHouseError> {
        if let Ok(value) = Self::parse_at(raw.trim(), depth) {
            return Ok(value);
        }
        top_level_whitespace(raw)
            .and_then(|index| Self::parse_at(raw[index..].trim(), depth).ok())
            .ok_or(ClickHouseError::UnsupportedType)
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
                Self::Boolean => match reader.read_u8().await? {
                    0 => Ok(OwnedValue::boolean(false)),
                    1 => Ok(OwnedValue::boolean(true)),
                    _ => Err(ClickHouseError::Protocol),
                },
                Self::Unsigned(bytes) => read_unsigned(reader, *bytes, limit).await,
                Self::Signed(bytes) => read_signed(reader, *bytes, limit).await,
                Self::BigInteger {
                    bytes,
                    signed,
                    type_name,
                } => read_big_integer(reader, *bytes, *signed, type_name, limit).await,
                Self::Decimal {
                    bytes,
                    scale,
                    type_name,
                } => read_decimal(reader, *bytes, *scale, type_name, limit).await,
                Self::Float32 => {
                    let mut bytes = [0; 4];
                    reader.read_exact(&mut bytes).await?;
                    Ok(OwnedValue::float64_bits(
                        f64::from(f32::from_bits(u32::from_le_bytes(bytes))).to_bits(),
                    ))
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
                    text_or_binary(bytes, truncation, limit)
                }
                Self::Binary(length) => {
                    let original = u64::try_from(*length).map_err(|_| ClickHouseError::Protocol)?;
                    let (bytes, truncation) = read_bounded(reader, original, limit).await?;
                    OwnedValue::binary(
                        BoundedBytes::from_vec(bytes, ByteLimit::new(limit))
                            .map_err(|_| ClickHouseError::Protocol)?,
                        truncation,
                    )
                    .map_err(|_| ClickHouseError::Protocol)
                }
                Self::Array(_) | Self::Tuple(_) | Self::Map(_, _) => {
                    let mut projection = StructuredProjection::new(limit);
                    self.read_projection(reader, &mut projection, limit).await?;
                    projection.finish()
                }
            }
        })
    }

    fn read_projection<'a>(
        &'a self,
        reader: &'a mut ChunkReader,
        output: &'a mut StructuredProjection,
        limit: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ClickHouseError>> + Send + 'a>>
    {
        Box::pin(async move {
            output.enter_node()?;
            match self {
                Self::Nullable(inner) => match reader.read_u8().await? {
                    0 => inner.read_projection(reader, output, limit).await,
                    1 => {
                        output.push("null");
                        Ok(())
                    }
                    _ => Err(ClickHouseError::Protocol),
                },
                Self::Array(inner) => {
                    let count = read_collection_len(reader).await?;
                    output.push("[");
                    for index in 0..count {
                        if index != 0 {
                            output.push(",");
                        }
                        inner.read_projection(reader, output, limit).await?;
                    }
                    output.push("]");
                    Ok(())
                }
                Self::Tuple(fields) => {
                    output.push("[");
                    for (index, field) in fields.iter().enumerate() {
                        if index != 0 {
                            output.push(",");
                        }
                        field.read_projection(reader, output, limit).await?;
                    }
                    output.push("]");
                    Ok(())
                }
                Self::Map(key, value) => {
                    let count = read_collection_len(reader).await?;
                    output.push("[");
                    for index in 0..count {
                        if index != 0 {
                            output.push(",");
                        }
                        output.push("[");
                        key.read_projection(reader, output, limit).await?;
                        output.push(",");
                        value.read_projection(reader, output, limit).await?;
                        output.push("]");
                    }
                    output.push("]");
                    Ok(())
                }
                scalar => {
                    let value = scalar.read(reader, limit).await?;
                    output.push_value(&value)
                }
            }
        })
    }
}

struct StructuredProjection {
    stored: String,
    original_byte_len: u64,
    limit: u64,
    saturated: bool,
    nodes: u64,
}

impl StructuredProjection {
    fn new(limit: u64) -> Self {
        Self {
            stored: String::new(),
            original_byte_len: 0,
            limit,
            saturated: false,
            nodes: 0,
        }
    }

    fn enter_node(&mut self) -> Result<(), ClickHouseError> {
        self.nodes = self.nodes.checked_add(1).ok_or(ClickHouseError::Protocol)?;
        if self.nodes > MAX_STRUCTURED_NODES {
            return Err(ClickHouseError::Protocol);
        }
        Ok(())
    }

    fn push(&mut self, value: &str) {
        self.original_byte_len = self
            .original_byte_len
            .saturating_add(u64::try_from(value.len()).unwrap_or(u64::MAX));
        if self.saturated {
            return;
        }
        let remaining = self
            .limit
            .saturating_sub(u64::try_from(self.stored.len()).unwrap_or(u64::MAX));
        let take = usize::try_from(remaining)
            .unwrap_or(usize::MAX)
            .min(value.len());
        let mut boundary = take;
        while !value.is_char_boundary(boundary) {
            boundary -= 1;
        }
        self.stored.push_str(&value[..boundary]);
        self.saturated = boundary != value.len();
    }

    fn push_quoted(&mut self, value: &str) {
        self.push("\"");
        for character in value.chars() {
            match character {
                '\"' => self.push("\\\""),
                '\\' => self.push("\\\\"),
                '\n' => self.push("\\n"),
                '\r' => self.push("\\r"),
                '\t' => self.push("\\t"),
                character if character.is_control() => {
                    self.push(&format!("\\u{:04x}", u32::from(character)));
                }
                character => self.push(character.encode_utf8(&mut [0; 4])),
            }
        }
        self.push("\"");
    }

    fn push_value(&mut self, value: &OwnedValue) -> Result<(), ClickHouseError> {
        match value.as_ref() {
            tablerock_core::ValueRef::Null => self.push("null"),
            tablerock_core::ValueRef::Boolean(value) => {
                self.push(if value { "true" } else { "false" })
            }
            tablerock_core::ValueRef::Signed(value) => self.push(&value.to_string()),
            tablerock_core::ValueRef::Unsigned(value) => self.push(&value.to_string()),
            tablerock_core::ValueRef::Float64Bits(value) => {
                self.push(&f64::from_bits(value).to_string());
            }
            tablerock_core::ValueRef::Decimal(value) => self.push(value),
            tablerock_core::ValueRef::Text { value, .. } => self.push_quoted(value),
            tablerock_core::ValueRef::Structured { value, .. } => self.push(value),
            tablerock_core::ValueRef::Binary { value, .. } => {
                self.push("{\"$binary\":\"");
                for byte in value {
                    self.push(&format!("{byte:02x}"));
                }
                self.push("\"}");
            }
            tablerock_core::ValueRef::Invalid { .. } | tablerock_core::ValueRef::Unknown { .. } => {
                return Err(ClickHouseError::Protocol);
            }
        }
        Ok(())
    }

    fn finish(self) -> Result<OwnedValue, ClickHouseError> {
        let stored_len = u64::try_from(self.stored.len()).unwrap_or(u64::MAX);
        let truncation = if stored_len == self.original_byte_len {
            Truncation::Complete
        } else {
            Truncation::Truncated {
                original_byte_len: Some(self.original_byte_len),
            }
        };
        OwnedValue::structured(
            BoundedText::from_string(self.stored, ByteLimit::new(self.limit))
                .map_err(|_| ClickHouseError::Protocol)?,
            truncation,
        )
        .map_err(|_| ClickHouseError::Protocol)
    }
}

fn text_or_binary(
    mut bytes: Vec<u8>,
    truncation: Truncation,
    limit: u64,
) -> Result<OwnedValue, ClickHouseError> {
    let text_len = match std::str::from_utf8(&bytes) {
        Ok(_) => Some(bytes.len()),
        Err(error)
            if matches!(truncation, Truncation::Truncated { .. })
                && error.error_len().is_none() =>
        {
            Some(error.valid_up_to())
        }
        Err(_) => None,
    };
    if let Some(text_len) = text_len {
        bytes.truncate(text_len);
        let text = String::from_utf8(bytes).map_err(|_| ClickHouseError::Protocol)?;
        return OwnedValue::text(
            BoundedText::from_string(text, ByteLimit::new(limit))
                .map_err(|_| ClickHouseError::Protocol)?,
            truncation,
        )
        .map_err(|_| ClickHouseError::Protocol);
    }
    OwnedValue::binary(
        BoundedBytes::from_vec(bytes, ByteLimit::new(limit))
            .map_err(|_| ClickHouseError::Protocol)?,
        truncation,
    )
    .map_err(|_| ClickHouseError::Protocol)
}

async fn read_collection_len(reader: &mut ChunkReader) -> Result<usize, ClickHouseError> {
    let count = read_leb128(reader).await?;
    if count > MAX_STRUCTURED_NODES {
        return Err(ClickHouseError::Protocol);
    }
    usize::try_from(count).map_err(|_| ClickHouseError::Protocol)
}

fn split_type_arguments(raw: &str) -> Result<Vec<&str>, ClickHouseError> {
    let mut arguments = Vec::new();
    let mut start = 0;
    let mut depth = 0_u32;
    let mut quoted = false;
    let mut escaped = false;
    for (index, character) in raw.char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '\'' {
                quoted = false;
            }
            continue;
        }
        match character {
            '\'' => quoted = true,
            '(' => depth = depth.checked_add(1).ok_or(ClickHouseError::Protocol)?,
            ')' => depth = depth.checked_sub(1).ok_or(ClickHouseError::Protocol)?,
            ',' if depth == 0 => {
                let argument = raw[start..index].trim();
                if argument.is_empty() {
                    return Err(ClickHouseError::Protocol);
                }
                arguments.push(argument);
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    if quoted || depth != 0 {
        return Err(ClickHouseError::Protocol);
    }
    let argument = raw[start..].trim();
    if argument.is_empty() {
        return if arguments.is_empty() {
            Ok(arguments)
        } else {
            Err(ClickHouseError::Protocol)
        };
    }
    arguments.push(argument);
    Ok(arguments)
}

fn top_level_whitespace(raw: &str) -> Option<usize> {
    let mut depth = 0_u32;
    let mut quoted = false;
    let mut escaped = false;
    for (index, character) in raw.char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '\'' {
                quoted = false;
            }
            continue;
        }
        match character {
            '\'' => quoted = true,
            '(' => depth = depth.saturating_add(1),
            ')' => depth = depth.saturating_sub(1),
            character if depth == 0 && character.is_whitespace() => return Some(index),
            _ => {}
        }
    }
    None
}

fn call_argument<'a>(raw: &'a str, name: &str) -> Option<&'a str> {
    raw.strip_prefix(name)
        .and_then(|raw| raw.strip_prefix('('))
        .and_then(|raw| raw.strip_suffix(')'))
}

fn split_pair(raw: &str) -> Result<(&str, &str), ClickHouseError> {
    let (left, right) = raw.split_once(',').ok_or(ClickHouseError::Protocol)?;
    if right.contains(',') {
        return Err(ClickHouseError::Protocol);
    }
    Ok((left.trim(), right.trim()))
}

fn decimal_scale(raw: &str, name: &str) -> Result<Option<u32>, ClickHouseError> {
    call_argument(raw, name)
        .map(|scale| scale.trim().parse().map_err(|_| ClickHouseError::Protocol))
        .transpose()
}

async fn read_unsigned(
    reader: &mut ChunkReader,
    byte_count: usize,
    limit: u64,
) -> Result<OwnedValue, ClickHouseError> {
    let mut bytes = [0_u8; 8];
    if byte_count > bytes.len() {
        return Err(ClickHouseError::Protocol);
    }
    reader.read_exact(&mut bytes[..byte_count]).await?;
    if limit >= 8 {
        Ok(OwnedValue::unsigned(u64::from_le_bytes(bytes)))
    } else {
        bounded_fixed_unknown(&bytes[..byte_count], "Unsigned", limit)
    }
}

async fn read_signed(
    reader: &mut ChunkReader,
    byte_count: usize,
    limit: u64,
) -> Result<OwnedValue, ClickHouseError> {
    let mut bytes = [0_u8; 8];
    if byte_count == 0 || byte_count > bytes.len() {
        return Err(ClickHouseError::Protocol);
    }
    reader.read_exact(&mut bytes[..byte_count]).await?;
    if bytes[byte_count - 1] & 0x80 != 0 {
        bytes[byte_count..].fill(0xff);
    }
    if limit >= 8 {
        Ok(OwnedValue::signed(i64::from_le_bytes(bytes)))
    } else {
        bounded_fixed_unknown(&bytes[..byte_count], "Signed", limit)
    }
}

async fn read_big_integer(
    reader: &mut ChunkReader,
    byte_count: usize,
    signed: bool,
    type_name: &str,
    limit: u64,
) -> Result<OwnedValue, ClickHouseError> {
    let bytes = read_fixed_bytes(reader, byte_count).await?;
    let text = integer_decimal(&bytes, signed)?;
    decimal_or_unknown(text, &bytes, type_name, limit)
}

async fn read_decimal(
    reader: &mut ChunkReader,
    byte_count: usize,
    scale: u32,
    type_name: &str,
    limit: u64,
) -> Result<OwnedValue, ClickHouseError> {
    let bytes = read_fixed_bytes(reader, byte_count).await?;
    let integer = integer_decimal(&bytes, true)?;
    let text = apply_decimal_scale(&integer, scale)?;
    decimal_or_unknown(text, &bytes, type_name, limit)
}

async fn read_fixed_bytes(
    reader: &mut ChunkReader,
    byte_count: usize,
) -> Result<Vec<u8>, ClickHouseError> {
    if byte_count == 0 || byte_count > 32 {
        return Err(ClickHouseError::Protocol);
    }
    let mut bytes = vec![0; byte_count];
    reader.read_exact(&mut bytes).await?;
    Ok(bytes)
}

fn decimal_or_unknown(
    text: String,
    raw: &[u8],
    type_name: &str,
    limit: u64,
) -> Result<OwnedValue, ClickHouseError> {
    if u64::try_from(text.len()).unwrap_or(u64::MAX) <= limit {
        let text = BoundedText::from_string(text, ByteLimit::new(limit))
            .map_err(|_| ClickHouseError::Protocol)?;
        return Ok(OwnedValue::decimal(text));
    }
    bounded_raw_unknown(raw, type_name, limit)
}

fn integer_decimal(bytes: &[u8], signed: bool) -> Result<String, ClickHouseError> {
    if bytes.is_empty() {
        return Err(ClickHouseError::Protocol);
    }
    let negative = signed && bytes.last().is_some_and(|byte| byte & 0x80 != 0);
    let mut magnitude = bytes.to_vec();
    if negative {
        let mut carry = 1_u16;
        for byte in &mut magnitude {
            let value = u16::from(!*byte) + carry;
            *byte = value as u8;
            carry = value >> 8;
        }
    }
    let mut digits = vec![0_u8];
    for byte in magnitude.iter().rev() {
        let mut carry = u16::from(*byte);
        for digit in &mut digits {
            let value = u16::from(*digit) * 256 + carry;
            *digit = (value % 10) as u8;
            carry = value / 10;
        }
        while carry != 0 {
            digits.push((carry % 10) as u8);
            carry /= 10;
        }
    }
    while digits.len() > 1 && digits.last() == Some(&0) {
        digits.pop();
    }
    let mut result = String::with_capacity(digits.len() + usize::from(negative));
    if negative && digits.iter().any(|digit| *digit != 0) {
        result.push('-');
    }
    result.extend(digits.iter().rev().map(|digit| char::from(b'0' + *digit)));
    Ok(result)
}

fn apply_decimal_scale(integer: &str, scale: u32) -> Result<String, ClickHouseError> {
    let scale = usize::try_from(scale).map_err(|_| ClickHouseError::Protocol)?;
    if scale == 0 {
        return Ok(integer.to_owned());
    }
    let (sign, digits) = integer
        .strip_prefix('-')
        .map_or(("", integer), |digits| ("-", digits));
    let padding = scale.saturating_sub(digits.len()) + usize::from(digits.len() <= scale);
    let mut result = String::with_capacity(sign.len() + padding + digits.len() + 1);
    result.push_str(sign);
    result.extend(std::iter::repeat_n('0', padding));
    let split = result.len() + digits.len() - scale;
    result.push_str(digits);
    result.insert(split, '.');
    Ok(result)
}

fn bounded_fixed_unknown(
    raw: &[u8],
    type_name: &str,
    limit: u64,
) -> Result<OwnedValue, ClickHouseError> {
    bounded_raw_unknown(raw, type_name, limit)
}

fn bounded_raw_unknown(
    raw: &[u8],
    type_name: &str,
    limit: u64,
) -> Result<OwnedValue, ClickHouseError> {
    let stored_len = usize::try_from(limit).unwrap_or(usize::MAX).min(raw.len());
    let payload = BoundedBytes::copy_from_slice(&raw[..stored_len], ByteLimit::new(limit))
        .map_err(|_| ClickHouseError::Protocol)?;
    let type_limit = u64::try_from(type_name.len()).unwrap_or(u64::MAX);
    let engine_type = EngineType::new(
        Engine::ClickHouse,
        BoundedText::copy_from_str(type_name, ByteLimit::new(type_limit))
            .map_err(|_| ClickHouseError::Protocol)?,
    )
    .map_err(|_| ClickHouseError::Protocol)?;
    OwnedValue::unknown(
        engine_type,
        payload,
        if stored_len == raw.len() {
            Truncation::Complete
        } else {
            Truncation::Truncated {
                original_byte_len: Some(raw.len() as u64),
            }
        },
    )
    .map_err(|_| ClickHouseError::Protocol)
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

#[cfg(test)]
mod tests {
    use tablerock_core::{OwnedValue, Truncation, ValueRef};

    use super::{
        ClickHouseType, StructuredProjection, apply_decimal_scale, integer_decimal,
        split_type_arguments, text_or_binary,
    };

    #[test]
    fn parses_recursive_and_named_container_types() {
        assert!(matches!(
            ClickHouseType::parse("Array(Nullable(UInt8))").unwrap(),
            ClickHouseType::Array(_)
        ));
        assert!(matches!(
            ClickHouseType::parse("Map(String, Array(Tuple(id UInt8, label String)))").unwrap(),
            ClickHouseType::Map(_, _)
        ));
        assert!(matches!(
            ClickHouseType::parse("Nested(id UInt8, label Nullable(String))").unwrap(),
            ClickHouseType::Array(_)
        ));
        assert_eq!(
            split_type_arguments("Enum8('a,b' = 1), Tuple(UInt8, String)").unwrap(),
            ["Enum8('a,b' = 1)", "Tuple(UInt8, String)"]
        );

        let deeply_nested = format!("{}UInt8{}", "Array(".repeat(65), ")".repeat(65));
        assert!(matches!(
            ClickHouseType::parse(&deeply_nested),
            Err(super::ClickHouseError::Protocol)
        ));
    }

    #[test]
    fn structured_projection_is_utf8_bounded_and_reports_full_length() {
        let mut projection = StructuredProjection::new(5);
        projection.push("[");
        projection.push_value(&OwnedValue::unsigned(7)).unwrap();
        projection.push(",");
        projection.push_quoted("é");
        projection.push("]");
        let value = projection.finish().unwrap();
        assert!(matches!(
            value.as_ref(),
            ValueRef::Structured {
                value: "[7,\"",
                truncation: Truncation::Truncated {
                    original_byte_len: Some(8)
                }
            }
        ));
    }

    #[test]
    fn converts_full_width_twos_complement_without_precision_loss() {
        assert_eq!(
            integer_decimal(&u128::MAX.to_le_bytes(), false).unwrap(),
            "340282366920938463463374607431768211455"
        );
        assert_eq!(
            integer_decimal(&i128::MIN.to_le_bytes(), true).unwrap(),
            "-170141183460469231731687303715884105728"
        );
    }

    #[test]
    fn places_decimal_scale_across_zero_boundary() {
        assert_eq!(apply_decimal_scale("123", 2).unwrap(), "1.23");
        assert_eq!(apply_decimal_scale("12", 2).unwrap(), "0.12");
        assert_eq!(apply_decimal_scale("-1", 2).unwrap(), "-0.01");
        assert_eq!(apply_decimal_scale("0", 9).unwrap(), "0.000000000");
    }

    #[test]
    fn truncates_text_only_at_a_utf8_boundary_but_preserves_invalid_binary() {
        let truncated = text_or_binary(
            vec![0xc3],
            Truncation::Truncated {
                original_byte_len: Some(2),
            },
            1,
        )
        .unwrap();
        assert!(matches!(
            truncated.as_ref(),
            ValueRef::Text {
                value: "",
                truncation: Truncation::Truncated {
                    original_byte_len: Some(2)
                }
            }
        ));

        let binary = text_or_binary(vec![0xff], Truncation::Complete, 1).unwrap();
        assert!(matches!(
            binary.as_ref(),
            ValueRef::Binary {
                value: [0xff],
                truncation: Truncation::Complete
            }
        ));
    }
}
