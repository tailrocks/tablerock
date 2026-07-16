use std::{error::Error, fmt, pin::Pin};

use futures_util::StreamExt;
use tablerock_core::{
    BoundedBytes, BoundedText, ByteLimit, ColumnMetadata, Engine, EngineType, OwnedValue,
    PageDelivery, PageFacts, PageIdentity, PageLimits, PageValidationError, PageWarning,
    PageWarnings, ResultPage, RowTotal, Truncation,
};
use tokio::task::JoinHandle;
use tokio::time::{Duration, sleep};
use tokio_postgres::{
    Row,
    config::SslMode,
    types::{FromSql, Type},
};
use tokio_postgres_rustls::MakeRustlsConnect;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PostgresTlsMode {
    Disable,
    Prefer,
    Require,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PostgresProbeQuery {
    BoundedSeries,
    TypedValues,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PostgresCancellationOutcome {
    ConfirmedByServer,
    RequestAcceptedButQueryCompleted,
}

impl PostgresProbeQuery {
    const fn sql(self) -> &'static str {
        match self {
            Self::BoundedSeries => {
                "SELECT value::text AS id, repeat('é', 10) AS label, NULL::text AS absent \
                 FROM generate_series(1, 3) AS value ORDER BY value"
            }
            Self::TypedValues => {
                "SELECT true::bool AS boolean_value, (-32768)::int2 AS int2_value, \
                 (-2147483648)::int4 AS int4_value, (-9223372036854775807)::int8 AS int8_value, \
                 1.5::float4 AS float4_value, '-0'::float8 AS float8_value, \
                 123.450::numeric AS numeric_value, repeat('é', 10)::text AS text_value, \
                 decode('0001ff', 'hex')::bytea AS binary_value, \
                 '123e4567-e89b-12d3-a456-426614174000'::uuid AS uuid_value, \
                 ARRAY[1, 2, 3]::int4[] AS array_value, NULL::uuid AS absent"
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct PostgresConnectConfig {
    host: BoundedText,
    port: u16,
    database: BoundedText,
    user: BoundedText,
    tls: PostgresTlsMode,
}

impl PostgresConnectConfig {
    #[must_use]
    pub const fn new(
        host: BoundedText,
        port: u16,
        database: BoundedText,
        user: BoundedText,
        tls: PostgresTlsMode,
    ) -> Self {
        Self {
            host,
            port,
            database,
            user,
            tls,
        }
    }
}

impl fmt::Debug for PostgresConnectConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PostgresConnectConfig")
            .field("host_bytes", &self.host.len())
            .field("port", &self.port)
            .field("database_bytes", &self.database.len())
            .field("user_bytes", &self.user.len())
            .field("tls", &self.tls)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostgresError {
    Connect,
    Query,
    Connection,
    Protocol,
    CancellationTransport,
    InvalidLimits,
    Page(PageValidationError),
}

impl fmt::Display for PostgresError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Connect => "PostgreSQL connection failed",
            Self::Query => "PostgreSQL query failed",
            Self::Connection => "PostgreSQL connection ended with an error",
            Self::Protocol => "PostgreSQL returned an unsupported result sequence",
            Self::CancellationTransport => "PostgreSQL cancellation transport failed",
            Self::InvalidLimits => "PostgreSQL stream limits are invalid",
            Self::Page(_) => "PostgreSQL result page failed validation",
        })
    }
}

impl Error for PostgresError {}

pub struct PostgresSession {
    client: tokio_postgres::Client,
    connection: JoinHandle<Result<(), PostgresError>>,
    tls: PostgresTlsMode,
}

impl PostgresSession {
    pub async fn connect(config: &PostgresConnectConfig) -> Result<Self, PostgresError> {
        let mut driver = tokio_postgres::Config::new();
        driver
            .host(config.host.as_str())
            .port(config.port)
            .dbname(config.database.as_str())
            .user(config.user.as_str())
            .ssl_mode(match config.tls {
                PostgresTlsMode::Disable => SslMode::Disable,
                PostgresTlsMode::Prefer => SslMode::Prefer,
                PostgresTlsMode::Require => SslMode::Require,
            });
        let (client, connection) = if config.tls == PostgresTlsMode::Disable {
            let (client, connection) = driver
                .connect(tokio_postgres::NoTls)
                .await
                .map_err(|_| PostgresError::Connect)?;
            let task =
                tokio::spawn(
                    async move { connection.await.map_err(|_| PostgresError::Connection) },
                );
            (client, task)
        } else {
            let (tls, _rejected_native_certificates) =
                MakeRustlsConnect::with_native_certs().map_err(|_| PostgresError::Connect)?;
            let (client, connection) = driver
                .connect(tls)
                .await
                .map_err(|_| PostgresError::Connect)?;
            let task =
                tokio::spawn(
                    async move { connection.await.map_err(|_| PostgresError::Connection) },
                );
            (client, task)
        };
        Ok(Self {
            client,
            connection,
            tls: config.tls,
        })
    }

    pub async fn stream_probe(
        &self,
        query: PostgresProbeQuery,
        limits: PageLimits,
        max_cell_bytes: u64,
    ) -> Result<PostgresRowStream, PostgresError> {
        if limits.max_rows() == 0
            || limits.max_columns() == 0
            || limits.max_arena_bytes() == 0
            || max_cell_bytes == 0
        {
            return Err(PostgresError::InvalidLimits);
        }
        let statement = self
            .client
            .prepare(query.sql())
            .await
            .map_err(|_| PostgresError::Query)?;
        let columns = decode_columns(statement.columns(), limits)?;
        let stream = self
            .client
            .query_raw(&statement, std::iter::empty::<&str>())
            .await
            .map_err(|_| PostgresError::Query)?;
        Ok(PostgresRowStream {
            stream: Box::pin(stream),
            pending: None,
            columns,
            limits,
            max_cell_bytes,
            complete: false,
        })
    }

    pub async fn shutdown(self) -> Result<(), PostgresError> {
        let Self {
            client,
            connection,
            tls: _,
        } = self;
        drop(client);
        connection.await.map_err(|_| PostgresError::Connection)??;
        Ok(())
    }

    pub async fn cancel_sleep_probe(&self) -> Result<PostgresCancellationOutcome, PostgresError> {
        let token = self.client.cancel_token();
        let query = self.client.simple_query("SELECT pg_sleep(30)");
        let cancellation = async {
            sleep(Duration::from_millis(150)).await;
            match self.tls {
                PostgresTlsMode::Disable => token.cancel_query(tokio_postgres::NoTls).await,
                PostgresTlsMode::Prefer | PostgresTlsMode::Require => {
                    let (tls, _rejected_native_certificates) =
                        MakeRustlsConnect::with_native_certs()
                            .map_err(|_| PostgresError::CancellationTransport)?;
                    token.cancel_query(tls).await
                }
            }
            .map_err(|_| PostgresError::CancellationTransport)
        };
        let (query_result, cancellation_result) = tokio::join!(query, cancellation);
        cancellation_result?;
        match query_result {
            Err(error)
                if error
                    .as_db_error()
                    .is_some_and(|error| error.code().code() == "57014") =>
            {
                Ok(PostgresCancellationOutcome::ConfirmedByServer)
            }
            Ok(_) => Ok(PostgresCancellationOutcome::RequestAcceptedButQueryCompleted),
            Err(_) => Err(PostgresError::Query),
        }
    }
}

pub struct PostgresRowStream {
    stream: Pin<Box<tokio_postgres::RowStream>>,
    pending: Option<Row>,
    columns: Vec<ColumnMetadata>,
    limits: PageLimits,
    max_cell_bytes: u64,
    complete: bool,
}

impl PostgresRowStream {
    pub async fn next_page(
        &mut self,
        identity: PageIdentity,
        start_row: u64,
    ) -> Result<Option<ResultPage>, PostgresError> {
        if self.complete {
            return Ok(None);
        }
        let mut values = Vec::new();
        let mut rows = 0_u32;
        let mut arena_used = 0_u64;
        let mut delivery = PageDelivery::Final;
        loop {
            let row = match self.pending.take() {
                Some(row) => Some(Ok(row)),
                None => self.stream.as_mut().next().await,
            };
            match row {
                Some(Ok(row)) => {
                    if row.len() != self.columns.len() {
                        return Err(PostgresError::Protocol);
                    }
                    if rows == self.limits.max_rows() {
                        self.pending = Some(row);
                        delivery = PageDelivery::Partial;
                        break;
                    }
                    arena_used += append_row(
                        &row,
                        &mut values,
                        self.max_cell_bytes,
                        self.limits.max_arena_bytes().saturating_sub(arena_used),
                    )?;
                    rows += 1;
                }
                None => {
                    self.complete = true;
                    break;
                }
                Some(Err(_)) => return Err(PostgresError::Query),
            }
        }
        let columns = self.columns.clone();
        let mut warnings = PageWarnings::none();
        if delivery == PageDelivery::Partial {
            warnings = warnings.with(PageWarning::RowLimitReached);
        }
        if values.iter().any(OwnedValue::is_truncated) {
            warnings = warnings.with(PageWarning::ByteLimitReached);
        }
        if values
            .iter()
            .any(|value| value.kind() == tablerock_core::ValueKind::Unknown)
        {
            warnings = warnings.with(PageWarning::UnknownValues);
        }
        if values
            .iter()
            .any(|value| value.kind() == tablerock_core::ValueKind::Invalid)
        {
            warnings = warnings.with(PageWarning::InvalidValues);
        }
        ResultPage::from_row_major(
            identity,
            start_row,
            RowTotal::Unknown,
            PageFacts::new(delivery, warnings),
            columns,
            values,
            self.limits,
        )
        .map(Some)
        .map_err(PostgresError::Page)
    }
}

fn decode_columns(
    columns: &[tokio_postgres::Column],
    limits: PageLimits,
) -> Result<Vec<ColumnMetadata>, PostgresError> {
    if columns.len() > limits.max_columns() as usize {
        return Err(PostgresError::Page(
            PageValidationError::ColumnLimitExceeded {
                actual: u32::try_from(columns.len()).unwrap_or(u32::MAX),
                limit: limits.max_columns(),
            },
        ));
    }
    columns
        .iter()
        .map(|column| {
            let name = BoundedText::copy_from_str(
                column.name(),
                ByteLimit::new(limits.max_column_text_bytes()),
            )
            .map_err(|_| PostgresError::Protocol)?;
            let engine_type = postgres_engine_type(column.type_(), limits.max_column_text_bytes())?;
            Ok(ColumnMetadata::new(name, engine_type, true))
        })
        .collect()
}

fn append_row(
    row: &Row,
    values: &mut Vec<OwnedValue>,
    max_cell_bytes: u64,
    mut arena_remaining: u64,
) -> Result<u64, PostgresError> {
    let initial_remaining = arena_remaining;
    for column in 0..row.len() {
        let Some(raw) = row
            .try_get::<_, Option<RawPostgresValue<'_>>>(column)
            .map_err(|_| PostgresError::Protocol)?
        else {
            values.push(OwnedValue::null());
            continue;
        };
        let byte_limit = max_cell_bytes.min(arena_remaining);
        let value = decode_value(row.columns()[column].type_(), raw.0, byte_limit)?;
        let stored_len = encoded_len(&value);
        values.push(value);
        arena_remaining = arena_remaining.saturating_sub(stored_len);
    }
    Ok(initial_remaining - arena_remaining)
}

struct RawPostgresValue<'a>(&'a [u8]);

impl<'a> FromSql<'a> for RawPostgresValue<'a> {
    fn from_sql(_type: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        Ok(Self(raw))
    }

    fn accepts(_type: &Type) -> bool {
        true
    }
}

fn postgres_engine_type(type_: &Type, limit: u64) -> Result<EngineType, PostgresError> {
    let name = BoundedText::copy_from_str(type_.name(), ByteLimit::new(limit))
        .map_err(|_| PostgresError::Protocol)?;
    EngineType::new(Engine::PostgreSql, name).map_err(|_| PostgresError::Protocol)
}

fn decode_value(type_: &Type, raw: &[u8], limit: u64) -> Result<OwnedValue, PostgresError> {
    let fixed = match *type_ {
        Type::BOOL if raw == [0] || raw == [1] => Some(OwnedValue::boolean(raw[0] != 0)),
        Type::INT2 if raw.len() == 2 => Some(OwnedValue::signed(
            i16::from_be_bytes([raw[0], raw[1]]) as i64,
        )),
        Type::INT4 if raw.len() == 4 => {
            Some(OwnedValue::signed(
                i32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]) as i64,
            ))
        }
        Type::INT8 if raw.len() == 8 => Some(OwnedValue::signed(i64::from_be_bytes([
            raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
        ]))),
        Type::FLOAT4 if raw.len() == 4 => Some(OwnedValue::float64_bits(
            f64::from(f32::from_bits(u32::from_be_bytes([
                raw[0], raw[1], raw[2], raw[3],
            ])))
            .to_bits(),
        )),
        Type::FLOAT8 if raw.len() == 8 => Some(OwnedValue::float64_bits(u64::from_be_bytes([
            raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
        ]))),
        _ => None,
    };
    if let Some(value) = fixed {
        return if encoded_len(&value) <= limit {
            Ok(value)
        } else {
            bounded_raw(type_, raw, 0, false)
        };
    }
    if matches!(
        *type_,
        Type::BOOL | Type::INT2 | Type::INT4 | Type::INT8 | Type::FLOAT4 | Type::FLOAT8
    ) {
        return bounded_raw(type_, raw, limit, true);
    }
    if type_.name() == "text"
        || type_.name() == "varchar"
        || type_.name() == "bpchar"
        || type_.name() == "name"
    {
        return match std::str::from_utf8(raw) {
            Ok(text) => {
                let stored_len = utf8_prefix(text, limit);
                let stored = BoundedText::copy_from_str(&text[..stored_len], ByteLimit::new(limit))
                    .map_err(|_| PostgresError::Protocol)?;
                OwnedValue::text(stored, truncation(stored_len, raw.len()))
                    .map_err(|_| PostgresError::Protocol)
            }
            Err(_) => bounded_raw(type_, raw, limit, true),
        };
    }
    if *type_ == Type::BYTEA {
        let stored_len = usize::try_from(limit).unwrap_or(usize::MAX).min(raw.len());
        let stored = BoundedBytes::copy_from_slice(&raw[..stored_len], ByteLimit::new(limit))
            .map_err(|_| PostgresError::Protocol)?;
        return OwnedValue::binary(stored, truncation(stored_len, raw.len()))
            .map_err(|_| PostgresError::Protocol);
    }
    bounded_raw(type_, raw, limit, false)
}

fn bounded_raw(
    type_: &Type,
    raw: &[u8],
    limit: u64,
    invalid: bool,
) -> Result<OwnedValue, PostgresError> {
    let stored_len = usize::try_from(limit).unwrap_or(usize::MAX).min(raw.len());
    let payload = BoundedBytes::copy_from_slice(&raw[..stored_len], ByteLimit::new(limit))
        .map_err(|_| PostgresError::Protocol)?;
    let engine_type =
        postgres_engine_type(type_, u64::try_from(type_.name().len()).unwrap_or(u64::MAX))?;
    if invalid {
        OwnedValue::invalid(engine_type, payload, truncation(stored_len, raw.len()))
    } else {
        OwnedValue::unknown(engine_type, payload, truncation(stored_len, raw.len()))
    }
    .map_err(|_| PostgresError::Protocol)
}

const fn truncation(stored_len: usize, original_len: usize) -> Truncation {
    if stored_len == original_len {
        Truncation::Complete
    } else {
        Truncation::Truncated {
            original_byte_len: Some(original_len as u64),
        }
    }
}

fn encoded_len(value: &OwnedValue) -> u64 {
    match value.as_ref() {
        tablerock_core::ValueRef::Null => 0,
        tablerock_core::ValueRef::Boolean(_) => 1,
        tablerock_core::ValueRef::Signed(_)
        | tablerock_core::ValueRef::Unsigned(_)
        | tablerock_core::ValueRef::Float64Bits(_) => 8,
        tablerock_core::ValueRef::Decimal(value)
        | tablerock_core::ValueRef::Text { value, .. }
        | tablerock_core::ValueRef::Structured { value, .. } => value.len() as u64,
        tablerock_core::ValueRef::Binary { value, .. }
        | tablerock_core::ValueRef::Invalid { payload: value, .. }
        | tablerock_core::ValueRef::Unknown { payload: value, .. } => value.len() as u64,
    }
}

fn utf8_prefix(value: &str, limit: u64) -> usize {
    let mut end = usize::try_from(limit)
        .unwrap_or(usize::MAX)
        .min(value.len());
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_prefix_never_splits_a_scalar() {
        assert_eq!(utf8_prefix("aéz", 0), 0);
        assert_eq!(utf8_prefix("aéz", 2), 1);
        assert_eq!(utf8_prefix("aéz", 3), 3);
        assert_eq!(utf8_prefix("aéz", 99), 4);
    }

    #[test]
    fn config_debug_exposes_only_lengths_and_transport_facts() {
        let config = PostgresConnectConfig::new(
            BoundedText::copy_from_str("SECRET_HOST", ByteLimit::new(64)).unwrap(),
            5432,
            BoundedText::copy_from_str("SECRET_DATABASE", ByteLimit::new(64)).unwrap(),
            BoundedText::copy_from_str("SECRET_USER", ByteLimit::new(64)).unwrap(),
            PostgresTlsMode::Require,
        );
        let debug = format!("{config:?}");
        for secret in ["SECRET_HOST", "SECRET_DATABASE", "SECRET_USER"] {
            assert!(!debug.contains(secret));
        }
        assert!(debug.contains("Require"));
    }

    #[test]
    fn malformed_known_payload_is_invalid_not_unknown() {
        let value = decode_value(&Type::BOOL, &[2], 8).unwrap();
        assert_eq!(value.kind(), tablerock_core::ValueKind::Invalid);
        assert_eq!(value.engine_type().unwrap().name(), "bool");
    }

    #[test]
    fn fixed_value_without_page_capacity_becomes_bounded_unknown() {
        let value = decode_value(&Type::INT8, &42_i64.to_be_bytes(), 4).unwrap();
        assert_eq!(value.kind(), tablerock_core::ValueKind::Unknown);
        assert!(matches!(
            value.as_ref(),
            tablerock_core::ValueRef::Unknown {
                payload: [],
                truncation: Truncation::Truncated {
                    original_byte_len: Some(8)
                },
                ..
            }
        ));
    }
}
