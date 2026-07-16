use std::{error::Error, fmt, pin::Pin};

use futures_util::StreamExt;
use tablerock_core::{
    BoundedText, ByteLimit, ColumnMetadata, Engine, EngineType, OwnedValue, PageDelivery,
    PageFacts, PageIdentity, PageLimits, PageValidationError, PageWarning, PageWarnings,
    ResultPage, RowTotal, Truncation,
};
use tokio::task::JoinHandle;
use tokio_postgres::{SimpleQueryMessage, config::SslMode};
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
}

impl PostgresProbeQuery {
    const fn sql(self) -> &'static str {
        match self {
            Self::BoundedSeries => {
                "SELECT value::text AS id, repeat('é', 10) AS label, NULL::text AS absent \
                 FROM generate_series(1, 3) AS value ORDER BY value"
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
            Self::InvalidLimits => "PostgreSQL stream limits are invalid",
            Self::Page(_) => "PostgreSQL result page failed validation",
        })
    }
}

impl Error for PostgresError {}

pub struct PostgresSession {
    client: tokio_postgres::Client,
    connection: JoinHandle<Result<(), PostgresError>>,
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
        Ok(Self { client, connection })
    }

    pub async fn stream_probe(
        &self,
        query: PostgresProbeQuery,
        limits: PageLimits,
        max_cell_bytes: u64,
    ) -> Result<PostgresTextStream, PostgresError> {
        if limits.max_rows() == 0
            || limits.max_columns() == 0
            || limits.max_arena_bytes() == 0
            || max_cell_bytes == 0
        {
            return Err(PostgresError::InvalidLimits);
        }
        let stream = self
            .client
            .simple_query_raw(query.sql())
            .await
            .map_err(|_| PostgresError::Query)?;
        Ok(PostgresTextStream {
            stream: Box::pin(stream),
            pending: None,
            columns: None,
            limits,
            max_cell_bytes,
            complete: false,
        })
    }

    pub async fn shutdown(self) -> Result<(), PostgresError> {
        let Self { client, connection } = self;
        drop(client);
        connection.await.map_err(|_| PostgresError::Connection)??;
        Ok(())
    }
}

pub struct PostgresTextStream {
    stream: Pin<Box<tokio_postgres::SimpleQueryStream>>,
    pending: Option<SimpleQueryMessage>,
    columns: Option<Vec<ColumnMetadata>>,
    limits: PageLimits,
    max_cell_bytes: u64,
    complete: bool,
}

impl PostgresTextStream {
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
            let message = match self.pending.take() {
                Some(message) => Some(Ok(message)),
                None => self.stream.as_mut().next().await,
            };
            match message {
                Some(Ok(SimpleQueryMessage::RowDescription(columns))) => {
                    if self.columns.is_some() || rows != 0 {
                        return Err(PostgresError::Protocol);
                    }
                    self.columns = Some(decode_columns(&columns, self.limits)?);
                }
                Some(Ok(SimpleQueryMessage::Row(row))) => {
                    let Some(columns) = &self.columns else {
                        return Err(PostgresError::Protocol);
                    };
                    if row.len() != columns.len() {
                        return Err(PostgresError::Protocol);
                    }
                    if rows == self.limits.max_rows() {
                        self.pending = Some(SimpleQueryMessage::Row(row));
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
                Some(Ok(SimpleQueryMessage::CommandComplete(_))) | None => {
                    self.complete = true;
                    break;
                }
                Some(Err(_)) => return Err(PostgresError::Query),
                Some(Ok(_)) => return Err(PostgresError::Protocol),
            }
        }
        let columns = self.columns.clone().ok_or(PostgresError::Protocol)?;
        let mut warnings = PageWarnings::none();
        if delivery == PageDelivery::Partial {
            warnings = warnings.with(PageWarning::RowLimitReached);
        }
        if values.iter().any(OwnedValue::is_truncated) {
            warnings = warnings.with(PageWarning::ByteLimitReached);
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
    columns: &[tokio_postgres::SimpleColumn],
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
            let engine_type = EngineType::new(
                Engine::PostgreSql,
                BoundedText::copy_from_str("text", ByteLimit::new(4))
                    .map_err(|_| PostgresError::Protocol)?,
            )
            .map_err(|_| PostgresError::Protocol)?;
            Ok(ColumnMetadata::new(name, engine_type, true))
        })
        .collect()
}

fn append_row(
    row: &tokio_postgres::SimpleQueryRow,
    values: &mut Vec<OwnedValue>,
    max_cell_bytes: u64,
    mut arena_remaining: u64,
) -> Result<u64, PostgresError> {
    let initial_remaining = arena_remaining;
    for column in 0..row.len() {
        let Some(value) = row.get(column) else {
            values.push(OwnedValue::null());
            continue;
        };
        let byte_limit = max_cell_bytes.min(arena_remaining);
        let stored_len = utf8_prefix(value, byte_limit);
        let stored = BoundedText::copy_from_str(&value[..stored_len], ByteLimit::new(byte_limit))
            .map_err(|_| PostgresError::Protocol)?;
        let truncation = if stored_len == value.len() {
            Truncation::Complete
        } else {
            Truncation::Truncated {
                original_byte_len: Some(value.len() as u64),
            }
        };
        values.push(OwnedValue::text(stored, truncation).map_err(|_| PostgresError::Protocol)?);
        arena_remaining = arena_remaining.saturating_sub(stored_len as u64);
    }
    Ok(initial_remaining - arena_remaining)
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
}
