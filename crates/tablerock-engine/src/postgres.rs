use std::{
    error::Error,
    fmt,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt, future::poll_fn};
use rustls::{
    ClientConfig, RootCertStore,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
};
use tablerock_core::{
    BoundedBytes, BoundedText, ByteLimit, ColumnMetadata, Engine, EngineType, OwnedValue,
    PageDelivery, PageFacts, PageIdentity, PageLimits, PageValidationError, PageWarning,
    PageWarnings, ResultPage, RowTotal, Truncation,
};
use tokio::task::JoinHandle;
use tokio::time::{Duration, sleep};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{Mutex, mpsc},
};
use tokio_postgres::{
    AsyncMessage, Connection, Row,
    config::SslMode,
    tls::MakeTlsConnect,
    types::{FromSql, ToSql, Type},
};
use tokio_postgres_rustls::MakeRustlsConnect;
use zeroize::Zeroize;

const MAX_TLS_MATERIAL_BYTES: usize = 65_536;
const MAX_CA_CERTIFICATES: usize = 16;
const MAX_CLIENT_CERTIFICATES: usize = 8;
const POSTGRES_NOTICE_QUEUE_CAPACITY: usize = 64;
const MAX_POSTGRES_NOTICE_SEVERITY_BYTES: u64 = 32;
const MAX_POSTGRES_NOTICE_CODE_BYTES: u64 = 5;
const MAX_POSTGRES_NOTICE_MESSAGE_BYTES: u64 = 1_024;

/// PostgreSQL transport-security requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PostgresTlsMode {
    /// Use a plaintext PostgreSQL transport.
    Disabled,
    /// Require a verified TLS handshake; plaintext fallback is forbidden.
    Required,
}

/// Borrowed client-certificate chain and unencrypted private-key material.
pub struct PostgresClientIdentity<'a> {
    certificate_chain_pem: &'a [u8],
    private_key_pem: &'a [u8],
}

impl<'a> PostgresClientIdentity<'a> {
    /// Creates an atomic client identity; validation occurs before connection.
    #[must_use]
    pub const fn new(certificate_chain_pem: &'a [u8], private_key_pem: &'a [u8]) -> Self {
        Self {
            certificate_chain_pem,
            private_key_pem,
        }
    }
}

impl fmt::Debug for PostgresClientIdentity<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PostgresClientIdentity")
            .field("certificate_chain_bytes", &self.certificate_chain_pem.len())
            .field("private_key_bytes", &self.private_key_pem.len())
            .finish()
    }
}

/// Borrowed custom-root material for one required-TLS connection.
pub struct PostgresTlsMaterial<'a> {
    ca_certificates_pem: &'a [u8],
    client_identity: Option<PostgresClientIdentity<'a>>,
}

impl<'a> PostgresTlsMaterial<'a> {
    /// Creates custom-root TLS material without client authentication.
    #[must_use]
    pub const fn new(ca_certificates_pem: &'a [u8]) -> Self {
        Self {
            ca_certificates_pem,
            client_identity: None,
        }
    }

    /// Adds the client certificate and private key as one atomic identity.
    #[must_use]
    pub const fn with_client_identity(mut self, identity: PostgresClientIdentity<'a>) -> Self {
        self.client_identity = Some(identity);
        self
    }
}

impl fmt::Debug for PostgresTlsMaterial<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PostgresTlsMaterial")
            .field("ca_certificate_bytes", &self.ca_certificates_pem.len())
            .field("has_client_identity", &self.client_identity.is_some())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PostgresProbeQuery {
    BoundedSeries,
    PerformanceSeries,
    TypedValues,
    Parameters,
    CancellationStream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PostgresCancellationOutcome {
    ConfirmedByServer,
    RequestAcceptedButQueryCompleted,
}

#[derive(Clone, PartialEq, Eq)]
pub struct PostgresNotice {
    severity: BoundedText,
    code: BoundedText,
    message: BoundedText,
    message_truncation: Truncation,
    detail: Option<BoundedText>,
    detail_truncation: Option<Truncation>,
    hint: Option<BoundedText>,
    hint_truncation: Option<Truncation>,
}

impl PostgresNotice {
    #[must_use]
    pub fn severity(&self) -> &str {
        self.severity.as_str()
    }

    #[must_use]
    pub fn code(&self) -> &str {
        self.code.as_str()
    }

    #[must_use]
    pub fn message(&self) -> &str {
        self.message.as_str()
    }

    #[must_use]
    pub const fn message_truncation(&self) -> Truncation {
        self.message_truncation
    }

    #[must_use]
    pub fn detail(&self) -> Option<&str> {
        self.detail.as_ref().map(BoundedText::as_str)
    }

    #[must_use]
    pub const fn detail_truncation(&self) -> Option<Truncation> {
        self.detail_truncation
    }

    #[must_use]
    pub fn hint(&self) -> Option<&str> {
        self.hint.as_ref().map(BoundedText::as_str)
    }

    #[must_use]
    pub const fn hint_truncation(&self) -> Option<Truncation> {
        self.hint_truncation
    }
}

impl fmt::Debug for PostgresNotice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PostgresNotice")
            .field("severity_bytes", &self.severity.len())
            .field("code_bytes", &self.code.len())
            .field("message_bytes", &self.message.len())
            .field(
                "message_truncated",
                &matches!(self.message_truncation, Truncation::Truncated { .. }),
            )
            .field("detail_bytes", &self.detail.as_ref().map(BoundedText::len))
            .field("hint_bytes", &self.hint.as_ref().map(BoundedText::len))
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostgresNoticeDelivery {
    Notice(PostgresNotice),
    Overflow { dropped: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PostgresStatementKind {
    Query,
    Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PostgresStatementOutcome {
    ordinal: u32,
    kind: PostgresStatementKind,
    row_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PostgresCopyLimits {
    max_chunks: u32,
    max_chunk_bytes: u64,
    max_total_bytes: u64,
}

impl PostgresCopyLimits {
    #[must_use]
    pub const fn new(max_chunks: u32, max_chunk_bytes: u64, max_total_bytes: u64) -> Self {
        Self {
            max_chunks,
            max_chunk_bytes,
            max_total_bytes,
        }
    }

    const fn is_valid(self) -> bool {
        self.max_chunks > 0 && self.max_chunk_bytes > 0 && self.max_total_bytes > 0
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct PostgresCopyChunk {
    ordinal: u32,
    byte_offset: u64,
    payload: BoundedBytes,
}

impl PostgresCopyChunk {
    #[must_use]
    pub const fn ordinal(&self) -> u32 {
        self.ordinal
    }

    #[must_use]
    pub const fn byte_offset(&self) -> u64 {
        self.byte_offset
    }

    #[must_use]
    pub fn payload(&self) -> &[u8] {
        self.payload.as_slice()
    }
}

impl fmt::Debug for PostgresCopyChunk {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PostgresCopyChunk")
            .field("ordinal", &self.ordinal)
            .field("byte_offset", &self.byte_offset)
            .field("payload_bytes", &self.payload.len())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PostgresCopyOutcome {
    chunk_count: u32,
    total_bytes: u64,
    row_count: Option<u64>,
}

impl PostgresCopyOutcome {
    #[must_use]
    pub const fn chunk_count(self) -> u32 {
        self.chunk_count
    }

    #[must_use]
    pub const fn total_bytes(self) -> u64 {
        self.total_bytes
    }

    #[must_use]
    pub const fn row_count(self) -> Option<u64> {
        self.row_count
    }
}

impl PostgresStatementOutcome {
    #[must_use]
    pub const fn ordinal(self) -> u32 {
        self.ordinal
    }

    #[must_use]
    pub const fn kind(self) -> PostgresStatementKind {
        self.kind
    }

    #[must_use]
    pub const fn row_count(self) -> u64 {
        self.row_count
    }
}

impl PostgresProbeQuery {
    const fn sql(self) -> &'static str {
        match self {
            Self::BoundedSeries => {
                "SELECT value::text AS id, repeat('é', 10) AS label, NULL::text AS absent \
                 FROM generate_series(1, 3) AS value ORDER BY value"
            }
            Self::PerformanceSeries => {
                "SELECT value::text AS id FROM generate_series(1, 10000) AS value ORDER BY value"
            }
            Self::TypedValues => {
                "SELECT true::bool AS boolean_value, (-32768)::int2 AS int2_value, \
                 (-2147483648)::int4 AS int4_value, (-9223372036854775807)::int8 AS int8_value, \
                 1.5::float4 AS float4_value, '-0'::float8 AS float8_value, \
                 123.450::numeric AS numeric_value, repeat('é', 10)::text AS text_value, \
                 decode('0001ff', 'hex')::bytea AS binary_value, \
                 '123e4567-e89b-12d3-a456-426614174000'::uuid AS uuid_value, \
                 ARRAY[1, 2, 3]::int4[] AS array_value, NULL::uuid AS absent, \
                 '{\"a\":[1,true]}'::json AS json_value, \
                 '{\"a\":[1,true]}'::jsonb AS jsonb_value, \
                 '[1,5)'::int4range AS range_value, \
                 ROW(7::int4, 'é'::text) AS composite_value, \
                 decode(repeat('ab', 16), 'hex')::bytea AS large_binary_value"
            }
            Self::Parameters => {
                "SELECT $1::text AS text_parameter, $2::int8 AS integer_parameter, \
                 $3::bytea AS binary_parameter, $4::bool AS boolean_parameter, \
                 $5::text AS null_parameter, $6::int4[] AS array_parameter"
            }
            Self::CancellationStream => {
                "SELECT value::text FROM generate_series(1, 1000) AS value UNION ALL \
                 SELECT 'blocked'::text FROM (SELECT pg_sleep(30)) AS delayed"
            }
        }
    }
}

/// Redacted PostgreSQL endpoint and transport configuration.
#[derive(Clone, PartialEq, Eq)]
pub struct PostgresConnectConfig {
    host: BoundedText,
    port: u16,
    database: BoundedText,
    user: BoundedText,
    tls: PostgresTlsMode,
    tls_server_name: Option<BoundedText>,
}

impl PostgresConnectConfig {
    /// Creates endpoint configuration without a separate TLS server name.
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
            tls_server_name: None,
        }
    }

    /// Overrides rustls verification/SNI without changing the network host.
    #[must_use]
    pub fn with_tls_server_name(mut self, tls_server_name: BoundedText) -> Self {
        self.tls_server_name = Some(tls_server_name);
        self
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
            .field(
                "tls_server_name_bytes",
                &self.tls_server_name.as_ref().map(BoundedText::len),
            )
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
    TlsConfiguration,
    ServerCancelled,
    InvalidLimits,
    CopyLimitExceeded,
    WriteOutcomeUnknown,
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
            Self::TlsConfiguration => "PostgreSQL TLS configuration is invalid",
            Self::ServerCancelled => "PostgreSQL server confirmed query cancellation",
            Self::InvalidLimits => "PostgreSQL stream limits are invalid",
            Self::CopyLimitExceeded => "PostgreSQL COPY limits were exceeded",
            Self::WriteOutcomeUnknown => "PostgreSQL write outcome is unknown",
            Self::Page(_) => "PostgreSQL result page failed validation",
        })
    }
}

impl Error for PostgresError {}

pub struct PostgresSession {
    client: tokio_postgres::Client,
    connection: JoinHandle<Result<(), PostgresError>>,
    transport: PostgresTransport,
    notices: Mutex<mpsc::Receiver<PostgresNotice>>,
    dropped_notices: Arc<AtomicU64>,
}

enum PostgresTransport {
    Plain,
    Rustls(PostgresRustlsConnector),
}

#[derive(Clone)]
struct PostgresRustlsConnector {
    inner: MakeRustlsConnect,
    server_name: Option<String>,
}

impl PostgresRustlsConnector {
    fn new(inner: MakeRustlsConnect, server_name: Option<&BoundedText>) -> Self {
        Self {
            inner,
            server_name: server_name.map(|name| name.as_str().to_owned()),
        }
    }
}

impl<S> MakeTlsConnect<S> for PostgresRustlsConnector
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    type Stream = <MakeRustlsConnect as MakeTlsConnect<S>>::Stream;
    type TlsConnect = <MakeRustlsConnect as MakeTlsConnect<S>>::TlsConnect;
    type Error = <MakeRustlsConnect as MakeTlsConnect<S>>::Error;

    fn make_tls_connect(&mut self, hostname: &str) -> Result<Self::TlsConnect, Self::Error> {
        <MakeRustlsConnect as MakeTlsConnect<S>>::make_tls_connect(
            &mut self.inner,
            self.server_name.as_deref().unwrap_or(hostname),
        )
    }
}

fn notice_channel() -> (
    mpsc::Sender<PostgresNotice>,
    mpsc::Receiver<PostgresNotice>,
    Arc<AtomicU64>,
) {
    let (sender, receiver) = mpsc::channel(POSTGRES_NOTICE_QUEUE_CAPACITY);
    (sender, receiver, Arc::new(AtomicU64::new(0)))
}

async fn drive_connection<S, T>(
    mut connection: Connection<S, T>,
    notices: mpsc::Sender<PostgresNotice>,
    dropped_notices: Arc<AtomicU64>,
) -> Result<(), PostgresError>
where
    S: AsyncRead + AsyncWrite + Unpin,
    T: AsyncRead + AsyncWrite + Unpin,
{
    loop {
        match poll_fn(|context| connection.poll_message(context)).await {
            Some(Ok(AsyncMessage::Notice(notice))) => {
                let notice = bounded_postgres_notice(&notice);
                match notices.try_send(notice) {
                    Ok(()) => {}
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        dropped_notices.fetch_add(1, Ordering::AcqRel);
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {}
                }
            }
            Some(Ok(_)) => {}
            Some(Err(_)) => return Err(PostgresError::Connection),
            None => return Ok(()),
        }
    }
}

fn bounded_postgres_notice(notice: &tokio_postgres::error::DbError) -> PostgresNotice {
    let (severity, _) = bounded_notice_text(notice.severity(), MAX_POSTGRES_NOTICE_SEVERITY_BYTES);
    let (code, _) = bounded_notice_text(notice.code().code(), MAX_POSTGRES_NOTICE_CODE_BYTES);
    let (message, message_truncation) =
        bounded_notice_text(notice.message(), MAX_POSTGRES_NOTICE_MESSAGE_BYTES);
    let (detail, detail_truncation) = bounded_optional_notice_text(notice.detail());
    let (hint, hint_truncation) = bounded_optional_notice_text(notice.hint());
    PostgresNotice {
        severity,
        code,
        message,
        message_truncation,
        detail,
        detail_truncation,
        hint,
        hint_truncation,
    }
}

fn bounded_optional_notice_text(value: Option<&str>) -> (Option<BoundedText>, Option<Truncation>) {
    match value {
        Some(value) => {
            let (value, truncation) = bounded_notice_text(value, MAX_POSTGRES_NOTICE_MESSAGE_BYTES);
            (Some(value), Some(truncation))
        }
        None => (None, None),
    }
}

fn bounded_notice_text(value: &str, max_bytes: u64) -> (BoundedText, Truncation) {
    let max_bytes = usize::try_from(max_bytes).unwrap_or(usize::MAX);
    let mut stored_bytes = value.len().min(max_bytes);
    while !value.is_char_boundary(stored_bytes) {
        stored_bytes -= 1;
    }
    let stored = &value[..stored_bytes];
    let bounded = BoundedText::copy_from_str(stored, ByteLimit::new(max_bytes as u64))
        .expect("notice truncation enforces its byte limit");
    let truncation = if stored_bytes == value.len() {
        Truncation::Complete
    } else {
        Truncation::Truncated {
            original_byte_len: Some(u64::try_from(value.len()).unwrap_or(u64::MAX)),
        }
    };
    (bounded, truncation)
}

impl PostgresSession {
    /// Connects with plaintext or native-root required TLS according to `config`.
    pub async fn connect(config: &PostgresConnectConfig) -> Result<Self, PostgresError> {
        match config.tls {
            PostgresTlsMode::Disabled => Self::connect_plain(config).await,
            PostgresTlsMode::Required => {
                let (connector, _rejected_native_certificates) =
                    MakeRustlsConnect::with_native_certs().map_err(|_| PostgresError::Connect)?;
                Self::connect_rustls(config, connector).await
            }
        }
    }

    /// Connects with required TLS using bounded custom roots and optional mTLS.
    pub async fn connect_with_tls(
        config: &PostgresConnectConfig,
        material: PostgresTlsMaterial<'_>,
    ) -> Result<Self, PostgresError> {
        if config.tls != PostgresTlsMode::Required {
            return Err(PostgresError::TlsConfiguration);
        }
        let connector = build_tls_connector(material)?;
        Self::connect_rustls(config, connector).await
    }

    async fn connect_plain(config: &PostgresConnectConfig) -> Result<Self, PostgresError> {
        let driver = driver_config(config);
        let (client, connection) = driver
            .connect(tokio_postgres::NoTls)
            .await
            .map_err(|_| PostgresError::Connect)?;
        let (notice_sender, notice_receiver, dropped_notices) = notice_channel();
        let connection = tokio::spawn(drive_connection(
            connection,
            notice_sender,
            Arc::clone(&dropped_notices),
        ));
        Ok(Self {
            client,
            connection,
            transport: PostgresTransport::Plain,
            notices: Mutex::new(notice_receiver),
            dropped_notices,
        })
    }

    async fn connect_rustls(
        config: &PostgresConnectConfig,
        connector: MakeRustlsConnect,
    ) -> Result<Self, PostgresError> {
        let connector = PostgresRustlsConnector::new(connector, config.tls_server_name.as_ref());
        let driver = driver_config(config);
        let (client, connection) = driver
            .connect(connector.clone())
            .await
            .map_err(|_| PostgresError::Connect)?;
        let (notice_sender, notice_receiver, dropped_notices) = notice_channel();
        let connection = tokio::spawn(drive_connection(
            connection,
            notice_sender,
            Arc::clone(&dropped_notices),
        ));
        Ok(Self {
            client,
            connection,
            transport: PostgresTransport::Rustls(connector),
            notices: Mutex::new(notice_receiver),
            dropped_notices,
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
        let text_parameter = "parameter-é";
        let integer_parameter = -9_223_372_036_854_775_000_i64;
        let binary_parameter = [0_u8, 1, 255, 0];
        let binary_parameter_slice: &[u8] = &binary_parameter;
        let boolean_parameter = false;
        let null_parameter: Option<&str> = None;
        let array_parameter = vec![1_i32, -2, 3];
        let parameters: Vec<&(dyn ToSql + Sync)> = match query {
            PostgresProbeQuery::Parameters => vec![
                &text_parameter,
                &integer_parameter,
                &binary_parameter_slice,
                &boolean_parameter,
                &null_parameter,
                &array_parameter,
            ],
            _ => Vec::new(),
        };
        let stream = self
            .client
            .query_raw(&statement, parameters)
            .await
            .map_err(|_| PostgresError::Query)?;
        Ok(PostgresRowStream {
            stream: Box::pin(stream),
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
            transport: _,
            notices: _,
            dropped_notices: _,
        } = self;
        drop(client);
        connection.await.map_err(|_| PostgresError::Connection)??;
        Ok(())
    }

    pub async fn next_notice(&self) -> Option<PostgresNoticeDelivery> {
        let dropped = self.dropped_notices.swap(0, Ordering::AcqRel);
        if dropped > 0 {
            return Some(PostgresNoticeDelivery::Overflow { dropped });
        }
        self.notices
            .lock()
            .await
            .recv()
            .await
            .map(PostgresNoticeDelivery::Notice)
    }

    pub async fn emit_notice_probe(&self) -> Result<(), PostgresError> {
        self.client
            .batch_execute(
                "DO $$ BEGIN RAISE NOTICE 'table-rock-notice' \
                 USING DETAIL = 'table-rock-detail', HINT = 'table-rock-hint'; END $$",
            )
            .await
            .map_err(|_| PostgresError::Query)
    }

    pub async fn emit_long_notice_probe(&self) -> Result<(), PostgresError> {
        self.client
            .batch_execute("DO $$ BEGIN RAISE NOTICE '%', repeat('é', 600); END $$")
            .await
            .map_err(|_| PostgresError::Query)
    }

    pub async fn emit_notice_overflow_probe(&self) -> Result<(), PostgresError> {
        self.client
            .batch_execute(
                "DO $$ BEGIN FOR notice_index IN 1..70 LOOP \
                 RAISE NOTICE 'table-rock-overflow-%', notice_index; END LOOP; END $$",
            )
            .await
            .map_err(|_| PostgresError::Query)
    }

    pub async fn multiple_statement_probe(
        &self,
    ) -> Result<Vec<PostgresStatementOutcome>, PostgresError> {
        let messages = self
            .client
            .simple_query_raw(
                "CREATE TEMP TABLE tablerock_statement_probe(value integer); \
                 INSERT INTO tablerock_statement_probe VALUES (1), (2); \
                 UPDATE tablerock_statement_probe SET value = 3 WHERE value = 2; \
                 SELECT value FROM tablerock_statement_probe ORDER BY value",
            )
            .await
            .map_err(|_| PostgresError::Query)?;
        tokio::pin!(messages);
        let mut outcomes = Vec::with_capacity(4);
        let mut has_rows = false;
        while let Some(message) = messages.next().await {
            match message.map_err(|_| PostgresError::Query)? {
                tokio_postgres::SimpleQueryMessage::RowDescription(_) => has_rows = true,
                tokio_postgres::SimpleQueryMessage::Row(_) => {}
                tokio_postgres::SimpleQueryMessage::CommandComplete(row_count) => {
                    let ordinal =
                        u32::try_from(outcomes.len()).map_err(|_| PostgresError::Query)?;
                    outcomes.push(PostgresStatementOutcome {
                        ordinal,
                        kind: if has_rows {
                            PostgresStatementKind::Query
                        } else {
                            PostgresStatementKind::Command
                        },
                        row_count,
                    });
                    has_rows = false;
                    if outcomes.len() > 4 {
                        return Err(PostgresError::Query);
                    }
                }
                _ => {}
            }
        }
        if outcomes.len() != 4 {
            return Err(PostgresError::Query);
        }
        Ok(outcomes)
    }

    pub async fn copy_out_probe(
        &self,
        limits: PostgresCopyLimits,
    ) -> Result<PostgresCopyOutStream, PostgresError> {
        if !limits.is_valid() {
            return Err(PostgresError::InvalidLimits);
        }
        let stream = self
            .client
            .copy_out(
                "COPY (SELECT value FROM generate_series(1, 1000) AS value ORDER BY value) \
                 TO STDOUT WITH (FORMAT csv)",
            )
            .await
            .map_err(|_| PostgresError::Query)?;
        Ok(PostgresCopyOutStream {
            stream: Box::pin(stream),
            limits,
            chunk_count: 0,
            total_bytes: 0,
            outcome: None,
            terminal: false,
        })
    }

    pub async fn copy_in_probe(
        &self,
        chunks: &[BoundedBytes],
        limits: PostgresCopyLimits,
    ) -> Result<PostgresCopyOutcome, PostgresError> {
        validate_copy_input(chunks, limits)?;
        self.client
            .batch_execute(
                "CREATE TEMP TABLE IF NOT EXISTS tablerock_copy_probe(value integer); \
                 TRUNCATE tablerock_copy_probe",
            )
            .await
            .map_err(|_| PostgresError::Query)?;
        let sink = self
            .client
            .copy_in::<_, Bytes>("COPY tablerock_copy_probe(value) FROM STDIN WITH (FORMAT csv)")
            .await
            .map_err(|_| PostgresError::Query)?;
        tokio::pin!(sink);
        let mut total_bytes = 0_u64;
        for chunk in chunks {
            total_bytes +=
                u64::try_from(chunk.len()).map_err(|_| PostgresError::CopyLimitExceeded)?;
            sink.as_mut()
                .send(Bytes::copy_from_slice(chunk.as_slice()))
                .await
                .map_err(|_| PostgresError::Query)?;
        }
        let row_count = sink
            .as_mut()
            .finish()
            .await
            .map_err(|_| PostgresError::Query)?;
        Ok(PostgresCopyOutcome {
            chunk_count: u32::try_from(chunks.len())
                .map_err(|_| PostgresError::CopyLimitExceeded)?,
            total_bytes,
            row_count: Some(row_count),
        })
    }

    pub async fn ambiguous_write_probe(&self) -> Result<(), PostgresError> {
        self.client
            .batch_execute(
                "CREATE TABLE IF NOT EXISTS tablerock_ambiguous_write_probe(\
                    sequence bigint GENERATED ALWAYS AS IDENTITY, marker integer NOT NULL\
                 ); TRUNCATE tablerock_ambiguous_write_probe RESTART IDENTITY",
            )
            .await
            .map_err(|_| PostgresError::Query)?;
        let write = self.client.execute(
            "WITH delay AS MATERIALIZED (SELECT pg_sleep(0.3)) \
             INSERT INTO tablerock_ambiguous_write_probe(marker) SELECT 1 FROM delay",
            &[],
        );
        match tokio::time::timeout(Duration::from_millis(100), write).await {
            Err(_) => Err(PostgresError::WriteOutcomeUnknown),
            Ok(Ok(_)) => Ok(()),
            Ok(Err(_)) => Err(PostgresError::Query),
        }
    }

    pub async fn ambiguous_write_count_probe(&self) -> Result<u64, PostgresError> {
        let row = self
            .client
            .query_one(
                "SELECT count(*)::bigint FROM tablerock_ambiguous_write_probe WHERE marker = 1",
                &[],
            )
            .await
            .map_err(|_| PostgresError::Query)?;
        let count: i64 = row.try_get(0).map_err(|_| PostgresError::Protocol)?;
        u64::try_from(count).map_err(|_| PostgresError::Protocol)
    }

    pub async fn ambiguous_commit_probe(&self) -> Result<(), PostgresError> {
        self.client
            .batch_execute(
                "CREATE TABLE tablerock_ambiguous_commit_probe(\
                    sequence bigint GENERATED ALWAYS AS IDENTITY, marker integer NOT NULL\
                 ); \
                 CREATE FUNCTION tablerock_delay_commit_probe() RETURNS trigger \
                 LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_sleep(1); RETURN NEW; END $$; \
                 CREATE CONSTRAINT TRIGGER tablerock_delay_commit_probe \
                 AFTER INSERT ON tablerock_ambiguous_commit_probe \
                 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW \
                 EXECUTE FUNCTION tablerock_delay_commit_probe()",
            )
            .await
            .map_err(|_| PostgresError::Query)?;
        let transaction = self.client.batch_execute(
            "BEGIN; \
             INSERT INTO tablerock_ambiguous_commit_probe(marker) VALUES (1); \
             COMMIT",
        );
        match tokio::time::timeout(Duration::from_millis(200), transaction).await {
            Err(_) => Err(PostgresError::WriteOutcomeUnknown),
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) => Err(PostgresError::Query),
        }
    }

    pub async fn ambiguous_commit_count_probe(&self) -> Result<u64, PostgresError> {
        let row = self
            .client
            .query_one(
                "SELECT count(*)::bigint FROM tablerock_ambiguous_commit_probe WHERE marker = 1",
                &[],
            )
            .await
            .map_err(|_| PostgresError::Query)?;
        let count: i64 = row.try_get(0).map_err(|_| PostgresError::Protocol)?;
        u64::try_from(count).map_err(|_| PostgresError::Protocol)
    }

    pub async fn prepare_ambiguous_transport_commit_probe(&self) -> Result<(), PostgresError> {
        self.client
            .batch_execute(
                "CREATE TABLE tablerock_ambiguous_transport_commit_probe(\
                    sequence bigint GENERATED ALWAYS AS IDENTITY, marker integer NOT NULL\
                 ); \
                 CREATE FUNCTION tablerock_delay_transport_commit_probe() RETURNS trigger \
                 LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_sleep(10); RETURN NEW; END $$; \
                 CREATE CONSTRAINT TRIGGER tablerock_delay_transport_commit_probe \
                 AFTER INSERT ON tablerock_ambiguous_transport_commit_probe \
                 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW \
                 EXECUTE FUNCTION tablerock_delay_transport_commit_probe()",
            )
            .await
            .map_err(|_| PostgresError::Query)
    }

    pub async fn ambiguous_transport_commit_probe(&self) -> Result<(), PostgresError> {
        self.client
            .batch_execute(
                "BEGIN; \
                 INSERT INTO tablerock_ambiguous_transport_commit_probe(marker) VALUES (1); \
                 COMMIT",
            )
            .await
            .map_err(|_| PostgresError::WriteOutcomeUnknown)
    }

    pub async fn ambiguous_transport_commit_waiting_probe(&self) -> Result<bool, PostgresError> {
        let row = self
            .client
            .query_one(
                "SELECT EXISTS(\
                    SELECT 1 FROM pg_stat_activity \
                    WHERE pid <> pg_backend_pid() \
                      AND state = 'active' \
                      AND wait_event = 'PgSleep' \
                      AND query LIKE '%tablerock_ambiguous_transport_commit_probe%'\
                 )",
                &[],
            )
            .await
            .map_err(|_| PostgresError::Query)?;
        row.try_get(0).map_err(|_| PostgresError::Protocol)
    }

    pub async fn ambiguous_transport_commit_count_probe(&self) -> Result<u64, PostgresError> {
        let row = self
            .client
            .query_one(
                "SELECT count(*)::bigint \
                 FROM tablerock_ambiguous_transport_commit_probe WHERE marker = 1",
                &[],
            )
            .await
            .map_err(|_| PostgresError::Query)?;
        let count: i64 = row.try_get(0).map_err(|_| PostgresError::Protocol)?;
        u64::try_from(count).map_err(|_| PostgresError::Protocol)
    }

    pub async fn cancel_sleep_probe(&self) -> Result<PostgresCancellationOutcome, PostgresError> {
        self.cancel_probe("SELECT pg_sleep(30)", Duration::from_millis(150))
            .await
    }

    pub async fn cancel_completed_probe(
        &self,
    ) -> Result<PostgresCancellationOutcome, PostgresError> {
        self.cancel_probe("SELECT 1", Duration::from_millis(250))
            .await
    }

    pub async fn cancel_transport_loss_probe(
        &self,
    ) -> Result<PostgresCancellationOutcome, PostgresError> {
        self.cancel_probe("SELECT pg_sleep(30)", Duration::from_secs(1))
            .await
    }

    async fn cancel_probe(
        &self,
        sql: &'static str,
        cancellation_delay: Duration,
    ) -> Result<PostgresCancellationOutcome, PostgresError> {
        let token = self.client.cancel_token();
        let query = self.client.simple_query(sql);
        let cancellation = async {
            sleep(cancellation_delay).await;
            match &self.transport {
                PostgresTransport::Plain => token.cancel_query(tokio_postgres::NoTls).await,
                PostgresTransport::Rustls(connector) => token.cancel_query(connector.clone()).await,
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
            Ok(_) => {
                self.synchronize_after_late_cancel().await?;
                Ok(PostgresCancellationOutcome::RequestAcceptedButQueryCompleted)
            }
            Err(_) => Err(PostgresError::Query),
        }
    }

    async fn synchronize_after_late_cancel(&self) -> Result<(), PostgresError> {
        match self.client.simple_query("SELECT pg_sleep(0.05)").await {
            Ok(_) => {}
            Err(error)
                if error
                    .as_db_error()
                    .is_some_and(|error| error.code().code() == "57014") => {}
            Err(_) => return Err(PostgresError::Query),
        }
        self.client
            .simple_query("SELECT 1")
            .await
            .map(|_| ())
            .map_err(|_| PostgresError::Query)
    }

    pub async fn dispatch_cancel(&self) -> Result<(), PostgresError> {
        let token = self.client.cancel_token();
        match &self.transport {
            PostgresTransport::Plain => token.cancel_query(tokio_postgres::NoTls).await,
            PostgresTransport::Rustls(connector) => token.cancel_query(connector.clone()).await,
        }
        .map_err(|_| PostgresError::CancellationTransport)
    }
}

pub struct PostgresCopyOutStream {
    stream: Pin<Box<tokio_postgres::CopyOutStream>>,
    limits: PostgresCopyLimits,
    chunk_count: u32,
    total_bytes: u64,
    outcome: Option<PostgresCopyOutcome>,
    terminal: bool,
}

impl PostgresCopyOutStream {
    pub async fn next_chunk(&mut self) -> Result<Option<PostgresCopyChunk>, PostgresError> {
        if self.terminal {
            return Ok(None);
        }
        let Some(payload) = self.stream.next().await else {
            self.terminal = true;
            self.outcome = Some(PostgresCopyOutcome {
                chunk_count: self.chunk_count,
                total_bytes: self.total_bytes,
                row_count: None,
            });
            return Ok(None);
        };
        let payload = payload.map_err(|_| {
            self.terminal = true;
            PostgresError::Query
        })?;
        let payload_bytes = u64::try_from(payload.len()).map_err(|_| {
            self.terminal = true;
            PostgresError::CopyLimitExceeded
        })?;
        let next_chunk_count = self.chunk_count.checked_add(1).ok_or_else(|| {
            self.terminal = true;
            PostgresError::CopyLimitExceeded
        })?;
        let next_total_bytes = self.total_bytes.checked_add(payload_bytes).ok_or_else(|| {
            self.terminal = true;
            PostgresError::CopyLimitExceeded
        })?;
        if next_chunk_count > self.limits.max_chunks
            || payload_bytes > self.limits.max_chunk_bytes
            || next_total_bytes > self.limits.max_total_bytes
        {
            self.terminal = true;
            return Err(PostgresError::CopyLimitExceeded);
        }
        let chunk = PostgresCopyChunk {
            ordinal: self.chunk_count,
            byte_offset: self.total_bytes,
            payload: BoundedBytes::copy_from_slice(
                payload.as_ref(),
                ByteLimit::new(self.limits.max_chunk_bytes),
            )
            .map_err(|_| PostgresError::CopyLimitExceeded)?,
        };
        self.chunk_count = next_chunk_count;
        self.total_bytes = next_total_bytes;
        Ok(Some(chunk))
    }

    #[must_use]
    pub const fn outcome(&self) -> Option<PostgresCopyOutcome> {
        self.outcome
    }
}

fn validate_copy_input(
    chunks: &[BoundedBytes],
    limits: PostgresCopyLimits,
) -> Result<(), PostgresError> {
    if !limits.is_valid() {
        return Err(PostgresError::InvalidLimits);
    }
    if chunks.len() > limits.max_chunks as usize {
        return Err(PostgresError::CopyLimitExceeded);
    }
    let mut total_bytes = 0_u64;
    for chunk in chunks {
        let chunk_bytes =
            u64::try_from(chunk.len()).map_err(|_| PostgresError::CopyLimitExceeded)?;
        if chunk_bytes > limits.max_chunk_bytes {
            return Err(PostgresError::CopyLimitExceeded);
        }
        total_bytes = total_bytes
            .checked_add(chunk_bytes)
            .ok_or(PostgresError::CopyLimitExceeded)?;
        if total_bytes > limits.max_total_bytes {
            return Err(PostgresError::CopyLimitExceeded);
        }
    }
    Ok(())
}

fn driver_config(config: &PostgresConnectConfig) -> tokio_postgres::Config {
    let mut driver = tokio_postgres::Config::new();
    driver
        .host(config.host.as_str())
        .port(config.port)
        .dbname(config.database.as_str())
        .user(config.user.as_str())
        .ssl_mode(match config.tls {
            PostgresTlsMode::Disabled => SslMode::Disable,
            PostgresTlsMode::Required => SslMode::Require,
        });
    driver
}

fn build_tls_connector(
    material: PostgresTlsMaterial<'_>,
) -> Result<MakeRustlsConnect, PostgresError> {
    let ca_certificates = parse_certificates(material.ca_certificates_pem, MAX_CA_CERTIFICATES)?;
    let mut roots = RootCertStore::empty();
    for certificate in ca_certificates {
        roots
            .add(certificate)
            .map_err(|_| PostgresError::TlsConfiguration)?;
    }
    let builder =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .map_err(|_| PostgresError::TlsConfiguration)?
            .with_root_certificates(roots);
    let config = match material.client_identity {
        Some(identity) => {
            let certificate_chain =
                parse_certificates(identity.certificate_chain_pem, MAX_CLIENT_CERTIFICATES)?;
            let mut keys = parse_private_keys(identity.private_key_pem)?;
            if keys.len() != 1 {
                keys.zeroize();
                return Err(PostgresError::TlsConfiguration);
            }
            builder
                .with_client_auth_cert(certificate_chain, keys.remove(0))
                .map_err(|_| PostgresError::TlsConfiguration)?
        }
        None => builder.with_no_client_auth(),
    };
    Ok(MakeRustlsConnect::new(config))
}

fn parse_certificates(
    pem: &[u8],
    maximum: usize,
) -> Result<Vec<CertificateDer<'static>>, PostgresError> {
    if pem.is_empty() || pem.len() > MAX_TLS_MATERIAL_BYTES {
        return Err(PostgresError::TlsConfiguration);
    }
    let certificates = CertificateDer::pem_slice_iter(pem)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| PostgresError::TlsConfiguration)?;
    if certificates.is_empty() || certificates.len() > maximum {
        return Err(PostgresError::TlsConfiguration);
    }
    Ok(certificates)
}

fn parse_private_keys(pem: &[u8]) -> Result<Vec<PrivateKeyDer<'static>>, PostgresError> {
    if pem.is_empty() || pem.len() > MAX_TLS_MATERIAL_BYTES {
        return Err(PostgresError::TlsConfiguration);
    }
    PrivateKeyDer::pem_slice_iter(pem)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| PostgresError::TlsConfiguration)
}

pub struct PostgresRowStream {
    stream: Pin<Box<tokio_postgres::RowStream>>,
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
            let row = self.stream.as_mut().next().await;
            match row {
                Some(Ok(row)) => {
                    if row.len() != self.columns.len() {
                        return Err(PostgresError::Protocol);
                    }
                    arena_used += append_row(
                        &row,
                        &mut values,
                        self.max_cell_bytes,
                        self.limits.max_arena_bytes().saturating_sub(arena_used),
                    )?;
                    rows += 1;
                    if rows == self.limits.max_rows() {
                        delivery = PageDelivery::Partial;
                        break;
                    }
                }
                None => {
                    self.complete = true;
                    break;
                }
                Some(Err(error)) => {
                    return Err(
                        if error
                            .as_db_error()
                            .is_some_and(|error| error.code().code() == "57014")
                        {
                            PostgresError::ServerCancelled
                        } else {
                            PostgresError::Query
                        },
                    );
                }
            }
        }
        if rows == 0 && self.complete {
            return Ok(None);
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
        let stored_len = value.encoded_byte_len();
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
        return if value.encoded_byte_len() <= limit {
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
            PostgresTlsMode::Required,
        )
        .with_tls_server_name(
            BoundedText::copy_from_str("SECRET_SERVER_NAME", ByteLimit::new(64)).unwrap(),
        );
        let debug = format!("{config:?}");
        for secret in [
            "SECRET_HOST",
            "SECRET_DATABASE",
            "SECRET_USER",
            "SECRET_SERVER_NAME",
        ] {
            assert!(!debug.contains(secret));
        }
        assert!(debug.contains("Required"));
    }

    #[test]
    fn tls_material_debug_never_exposes_certificate_or_key_bytes() {
        let material = PostgresTlsMaterial::new(b"SECRET_CA").with_client_identity(
            PostgresClientIdentity::new(b"SECRET_CERT", b"SECRET_PRIVATE_KEY"),
        );
        let debug = format!("{material:?}");
        for secret in ["SECRET_CA", "SECRET_CERT", "SECRET_PRIVATE_KEY"] {
            assert!(!debug.contains(secret));
        }
        assert!(debug.contains("has_client_identity: true"));
    }

    #[test]
    fn tls_material_rejects_empty_and_malformed_roots() {
        assert!(matches!(
            build_tls_connector(PostgresTlsMaterial::new(b"")),
            Err(PostgresError::TlsConfiguration)
        ));
        assert!(matches!(
            build_tls_connector(PostgresTlsMaterial::new(b"not PEM")),
            Err(PostgresError::TlsConfiguration)
        ));
    }

    #[tokio::test]
    async fn custom_tls_material_requires_the_require_mode() {
        let config = PostgresConnectConfig::new(
            BoundedText::copy_from_str("localhost", ByteLimit::new(64)).unwrap(),
            5432,
            BoundedText::copy_from_str("postgres", ByteLimit::new(64)).unwrap(),
            BoundedText::copy_from_str("postgres", ByteLimit::new(64)).unwrap(),
            PostgresTlsMode::Disabled,
        );
        assert!(matches!(
            PostgresSession::connect_with_tls(
                &PostgresConnectConfig::new(
                    config.host.clone(),
                    config.port,
                    config.database.clone(),
                    config.user.clone(),
                    PostgresTlsMode::Disabled,
                ),
                PostgresTlsMaterial::new(b"not reached"),
            )
            .await,
            Err(PostgresError::TlsConfiguration)
        ));
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
