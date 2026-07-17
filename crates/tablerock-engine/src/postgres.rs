use std::{
    error::Error,
    fmt,
    io::Write,
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
    PageWarnings, ResultPage, RowTotal, Truncation, ValueRef,
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
    types::{Field, FromSql, Kind, ToSql, Type},
};
use tokio_postgres_rustls::MakeRustlsConnect;
use zeroize::Zeroize;

use crate::temporal::format_date_from_unix_days;

const MAX_TLS_MATERIAL_BYTES: usize = 65_536;
const MAX_CA_CERTIFICATES: usize = 16;
const MAX_CLIENT_CERTIFICATES: usize = 8;
const POSTGRES_NOTICE_QUEUE_CAPACITY: usize = 64;
const MAX_POSTGRES_NOTICE_SEVERITY_BYTES: u64 = 32;
const MAX_POSTGRES_NOTICE_CODE_BYTES: u64 = 5;
const MAX_POSTGRES_NOTICE_MESSAGE_BYTES: u64 = 1_024;
const MAX_POSTGRES_ARRAY_DIMENSIONS: usize = 64;
const MAX_POSTGRES_ARRAY_ELEMENTS: usize = 1_000_000;
const MAX_POSTGRES_COMPOSITE_FIELDS: usize = 1_664;
const MAX_POSTGRES_NESTING_DEPTH: usize = 64;

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
    NumericValues,
    UuidValues,
    TemporalValues,
    ArrayValues,
    RangeValues,
    MultirangeValues,
    CompositeValues,
    DomainValues,
    EnumValues,
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
            Self::NumericValues => {
                "SELECT 123.450::numeric AS positive_scaled, \
                 (-0.0012300)::numeric AS negative_scaled, \
                 12345678901234567890.1234567890::numeric AS arbitrary_precision, \
                 'NaN'::numeric AS not_a_number, 'Infinity'::numeric AS positive_infinity, \
                 '-Infinity'::numeric AS negative_infinity, 0.000::numeric AS scaled_zero"
            }
            Self::UuidValues => {
                "SELECT '123e4567-e89b-12d3-a456-426614174000'::uuid AS representative, \
                 '00000000-0000-0000-0000-000000000000'::uuid AS nil, \
                 'ffffffff-ffff-ffff-ffff-ffffffffffff'::uuid AS maximum"
            }
            Self::TemporalValues => {
                "SELECT DATE '2000-01-01' AS epoch_date, DATE '2024-02-29' AS leap_date, \
                 TIME '24:00:00' AS end_of_day, TIME '12:34:56.123456' AS precise_time, \
                 TIMESTAMP '1999-12-31 23:59:59.999999' AS local_timestamp, \
                 TIMESTAMPTZ '2024-02-29 12:34:56.123456+07' AS utc_timestamp, \
                 'infinity'::date AS infinite_date, '-infinity'::timestamptz AS negative_infinity, \
                 TIMETZ '12:34:56.123456+06:30' AS zoned_time, \
                 INTERVAL '14 months -3 days -14706.123456 seconds' AS mixed_interval, \
                 DATE '0001-01-01 BC' AS bc_date, DATE '10000-12-31' AS expanded_date, \
                 'infinity'::interval AS infinite_interval, \
                 '-infinity'::interval AS negative_infinite_interval"
            }
            Self::ArrayValues => {
                "SELECT ARRAY[1, NULL, -2]::int4[] AS nullable_vector, \
                 ARRAY[[1, 2], [3, 4]]::int4[] AS matrix, \
                 '[0:2]={7,8,9}'::int4[] AS zero_based, \
                 ARRAY['plain', 'quoted\"', 'NULL', 'é']::text[] AS text_vector, \
                 ARRAY[DATE '2024-02-29', DATE '2000-01-01']::date[] AS date_vector, \
                 ARRAY['[1,3)'::int4range, 'empty'::int4range] AS range_vector"
            }
            Self::RangeValues => {
                "SELECT '[1,5)'::int4range AS integer_range, \
                 '(,42]'::int8range AS unbounded_range, \
                 '(1.20,2.30]'::numrange AS numeric_range, \
                 '[2024-02-29,2024-03-02)'::daterange AS date_range, \
                 '[2024-02-29 12:00:00+07,2024-02-29 13:00:00+07)'::tstzrange \
                    AS timestamp_range, \
                 'empty'::tstzrange AS empty_range"
            }
            Self::MultirangeValues => {
                "SELECT '{}'::int4multirange AS empty_multirange, \
                 '{[1,3),[5,8)}'::int4multirange AS integer_multirange, \
                 '{(,0),[10,)}'::int8multirange AS unbounded_multirange, \
                 '{(1.20,2.30],[5.00,6.00)}'::nummultirange AS numeric_multirange, \
                 '{[2024-02-29,2024-03-02),[2024-03-10,2024-03-11)}'::datemultirange \
                    AS date_multirange"
            }
            Self::CompositeValues => {
                "SELECT ROW(7, 'é', NULL, ARRAY[1,2], '[2024-02-29,2024-03-02)'::daterange) \
                    ::tablerock_composite_probe AS named_composite, \
                 ROW(7::int4, 'é'::text, NULL::text, ARRAY[1,2]::int4[]) AS anonymous_record"
            }
            Self::DomainValues => {
                "SELECT ROW(\
                    7, 8, ARRAY[1,2], '[2024-02-29,2024-03-02)'::daterange,\
                    ROW(9, 'domain', NULL, ARRAY[3,4],\
                        '[2024-03-10,2024-03-11)'::daterange\
                    )::tablerock_composite_probe\
                 )::tablerock_domain_container AS domain_container"
            }
            Self::EnumValues => {
                "SELECT 'ready'::tablerock_status AS ascii_label, \
                 'café'::tablerock_status AS unicode_label"
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

    pub async fn prepare_composite_probe(&self) -> Result<(), PostgresError> {
        self.client
            .batch_execute(
                "CREATE TYPE tablerock_composite_probe AS (\
                    id int4, label text, absent text, numbers int4[], span daterange\
                )",
            )
            .await
            .map_err(|_| PostgresError::Query)
    }

    pub async fn prepare_domain_probe(&self) -> Result<(), PostgresError> {
        self.client
            .batch_execute(
                "CREATE DOMAIN tablerock_positive AS int4 CHECK (VALUE > 0);\
                 CREATE DOMAIN tablerock_nested_positive AS tablerock_positive;\
                 CREATE DOMAIN tablerock_ints AS int4[];\
                 CREATE DOMAIN tablerock_dates AS daterange;\
                 CREATE DOMAIN tablerock_composite_domain AS tablerock_composite_probe;\
                 CREATE TYPE tablerock_domain_container AS (\
                    positive_domain tablerock_positive,\
                    nested_domain tablerock_nested_positive,\
                    array_domain tablerock_ints,\
                    range_domain tablerock_dates,\
                    composite_domain tablerock_composite_domain\
                 )",
            )
            .await
            .map_err(|_| PostgresError::Query)
    }

    pub async fn prepare_enum_probe(&self) -> Result<(), PostgresError> {
        self.client
            .batch_execute("CREATE TYPE tablerock_status AS ENUM ('ready', 'café', 'blocked')")
            .await
            .map_err(|_| PostgresError::Query)
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
    decode_value_at_depth(type_, raw, limit, 0)
}

fn decode_value_at_depth(
    type_: &Type,
    raw: &[u8],
    limit: u64,
    nesting_depth: usize,
) -> Result<OwnedValue, PostgresError> {
    if nesting_depth > MAX_POSTGRES_NESTING_DEPTH {
        return bounded_raw(type_, raw, limit, false);
    }
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
    if *type_ == Type::JSON || *type_ == Type::JSONB {
        return decode_json(type_, raw, limit);
    }
    if *type_ == Type::NUMERIC {
        return decode_numeric(type_, raw, limit);
    }
    if *type_ == Type::UUID {
        return decode_uuid(type_, raw, limit);
    }
    if matches!(
        *type_,
        Type::DATE
            | Type::TIME
            | Type::TIMETZ
            | Type::TIMESTAMP
            | Type::TIMESTAMPTZ
            | Type::INTERVAL
    ) {
        return decode_temporal(type_, raw, limit);
    }
    if let Kind::Enum(variants) = type_.kind() {
        let Ok(label) = std::str::from_utf8(raw) else {
            return bounded_raw(type_, raw, limit, true);
        };
        if !variants.iter().any(|variant| variant == label) {
            return bounded_raw(type_, raw, limit, true);
        }
        let stored_len = utf8_prefix(label, limit);
        let stored = BoundedText::copy_from_str(&label[..stored_len], ByteLimit::new(limit))
            .map_err(|_| PostgresError::Protocol)?;
        return OwnedValue::text(stored, truncation(stored_len, raw.len()))
            .map_err(|_| PostgresError::Protocol);
    }
    if let Kind::Domain(underlying_type) = type_.kind() {
        let value =
            decode_value_at_depth(underlying_type, raw, limit, nesting_depth.saturating_add(1))?;
        return match value.as_ref() {
            ValueRef::Invalid { .. } => bounded_raw(type_, raw, limit, true),
            ValueRef::Unknown { .. } => bounded_raw(type_, raw, limit, false),
            _ => Ok(value),
        };
    }
    if let Kind::Array(element_type) = type_.kind() {
        return match decode_array(element_type, raw, limit, nesting_depth) {
            Ok(Some(value)) => Ok(value),
            Ok(None) => bounded_raw(type_, raw, limit, false),
            Err(()) => bounded_raw(type_, raw, limit, true),
        };
    }
    if let Kind::Range(element_type) = type_.kind() {
        return match decode_range(element_type, raw, limit, nesting_depth) {
            Ok(Some(value)) => Ok(value),
            Ok(None) => bounded_raw(type_, raw, limit, false),
            Err(()) => bounded_raw(type_, raw, limit, true),
        };
    }
    if let Kind::Multirange(element_type) = type_.kind() {
        return match decode_multirange(element_type, raw, limit, nesting_depth) {
            Ok(Some(value)) => Ok(value),
            Ok(None) => bounded_raw(type_, raw, limit, false),
            Err(()) => bounded_raw(type_, raw, limit, true),
        };
    }
    if let Kind::Composite(fields) = type_.kind() {
        return match decode_composite(Some(fields), raw, limit, nesting_depth) {
            Ok(Some(value)) => Ok(value),
            Ok(None) => bounded_raw(type_, raw, limit, false),
            Err(()) => bounded_raw(type_, raw, limit, true),
        };
    }
    if *type_ == Type::RECORD {
        return match decode_composite(None, raw, limit, nesting_depth) {
            Ok(Some(value)) => Ok(value),
            Ok(None) => bounded_raw(type_, raw, limit, false),
            Err(()) => bounded_raw(type_, raw, limit, true),
        };
    }
    bounded_raw(type_, raw, limit, false)
}

fn decode_multirange(
    element_type: &Type,
    raw: &[u8],
    limit: u64,
    nesting_depth: usize,
) -> Result<Option<OwnedValue>, ()> {
    let mut cursor = PostgresBinaryCursor::new(raw);
    let count = usize::try_from(cursor.read_u32()?).map_err(|_| ())?;
    if count > MAX_POSTGRES_ARRAY_ELEMENTS {
        return Ok(None);
    }
    let mut projection = BoundedJsonWriter::new(limit);
    projection.push("{\"$multirange\":[")?;
    let component_limit = u64::try_from(MAX_JSON_INPUT_BYTES).unwrap_or(u64::MAX);
    for index in 0..count {
        if index != 0 {
            projection.push(",")?;
        }
        let length = usize::try_from(cursor.read_u32()?).map_err(|_| ())?;
        let payload = cursor.read_exact(length)?;
        let value = match decode_range(
            element_type,
            payload,
            component_limit,
            nesting_depth.saturating_add(1),
        ) {
            Ok(Some(value)) => value,
            Ok(None) => return Ok(None),
            Err(()) => return Err(()),
        };
        if value.is_truncated() || !project_structured_value(&value, &mut projection)? {
            return Ok(None);
        }
    }
    if cursor.remaining() != 0 {
        return Err(());
    }
    projection.push("]}")?;
    projection.finish().map(Some).map_err(|_| ())
}

fn decode_composite(
    declared_fields: Option<&[Field]>,
    raw: &[u8],
    limit: u64,
    nesting_depth: usize,
) -> Result<Option<OwnedValue>, ()> {
    let mut cursor = PostgresBinaryCursor::new(raw);
    let field_count = usize::try_from(cursor.read_u32()?).map_err(|_| ())?;
    if field_count > MAX_POSTGRES_COMPOSITE_FIELDS {
        return Ok(None);
    }
    if declared_fields.is_some_and(|fields| fields.len() != field_count) {
        return Err(());
    }
    let mut projection = BoundedJsonWriter::new(limit);
    projection.push("{\"$composite\":{\"fields\":[")?;
    let component_limit = u64::try_from(MAX_JSON_INPUT_BYTES).unwrap_or(u64::MAX);
    for index in 0..field_count {
        if index != 0 {
            projection.push(",")?;
        }
        let wire_oid = cursor.read_u32()?;
        let (name, field_type) = if let Some(fields) = declared_fields {
            let field = &fields[index];
            if wire_oid != field.type_().oid() {
                return Err(());
            }
            (Some(field.name()), field.type_().clone())
        } else {
            let Some(field_type) = Type::from_oid(wire_oid) else {
                return Ok(None);
            };
            (None, field_type)
        };
        projection.push("{\"name\":")?;
        match name {
            Some(name) => projection.push_json_string(name)?,
            None => projection.push("null")?,
        }
        projection.push(&format!(",\"oid\":{wire_oid},\"type\":"))?;
        projection.push_json_string(field_type.name())?;
        projection.push(",\"value\":")?;
        let length = cursor.read_i32()?;
        if length == -1 {
            projection.push("null}")?;
            continue;
        }
        let length = usize::try_from(length).map_err(|_| ())?;
        let payload = cursor.read_exact(length)?;
        let value = decode_value_at_depth(
            &field_type,
            payload,
            component_limit,
            nesting_depth.saturating_add(1),
        )
        .map_err(|_| ())?;
        if matches!(value.as_ref(), ValueRef::Invalid { .. }) {
            return Err(());
        }
        if value.is_truncated()
            || matches!(value.as_ref(), ValueRef::Unknown { .. })
            || !project_structured_value(&value, &mut projection)?
        {
            return Ok(None);
        }
        projection.push("}")?;
    }
    if cursor.remaining() != 0 {
        return Err(());
    }
    projection.push("]}}")?;
    projection.finish().map(Some).map_err(|_| ())
}

const RANGE_EMPTY: u8 = 0x01;
const RANGE_LOWER_INCLUSIVE: u8 = 0x02;
const RANGE_UPPER_INCLUSIVE: u8 = 0x04;
const RANGE_LOWER_UNBOUNDED: u8 = 0x08;
const RANGE_UPPER_UNBOUNDED: u8 = 0x10;
const RANGE_KNOWN_FLAGS: u8 = RANGE_EMPTY
    | RANGE_LOWER_INCLUSIVE
    | RANGE_UPPER_INCLUSIVE
    | RANGE_LOWER_UNBOUNDED
    | RANGE_UPPER_UNBOUNDED;

fn decode_range(
    element_type: &Type,
    raw: &[u8],
    limit: u64,
    nesting_depth: usize,
) -> Result<Option<OwnedValue>, ()> {
    let mut cursor = PostgresBinaryCursor::new(raw);
    let flags = cursor.read_u8()?;
    if flags & !RANGE_KNOWN_FLAGS != 0 {
        return Err(());
    }
    let mut projection = BoundedJsonWriter::new(limit);
    if flags == RANGE_EMPTY {
        if cursor.remaining() != 0 {
            return Err(());
        }
        projection.push("{\"$range\":{\"empty\":true}}")?;
        return projection.finish().map(Some).map_err(|_| ());
    }
    if flags & RANGE_EMPTY != 0
        || flags & RANGE_LOWER_UNBOUNDED != 0 && flags & RANGE_LOWER_INCLUSIVE != 0
        || flags & RANGE_UPPER_UNBOUNDED != 0 && flags & RANGE_UPPER_INCLUSIVE != 0
    {
        return Err(());
    }
    projection.push("{\"$range\":{\"empty\":false,\"lower\":")?;
    if !project_range_bound(
        element_type,
        &mut cursor,
        &mut projection,
        nesting_depth,
        flags & RANGE_LOWER_UNBOUNDED != 0,
        flags & RANGE_LOWER_INCLUSIVE != 0,
    )? {
        return Ok(None);
    }
    projection.push(",\"upper\":")?;
    if !project_range_bound(
        element_type,
        &mut cursor,
        &mut projection,
        nesting_depth,
        flags & RANGE_UPPER_UNBOUNDED != 0,
        flags & RANGE_UPPER_INCLUSIVE != 0,
    )? {
        return Ok(None);
    }
    if cursor.remaining() != 0 {
        return Err(());
    }
    projection.push("}}")?;
    projection.finish().map(Some).map_err(|_| ())
}

fn project_range_bound(
    element_type: &Type,
    cursor: &mut PostgresBinaryCursor<'_>,
    projection: &mut BoundedJsonWriter,
    nesting_depth: usize,
    unbounded: bool,
    inclusive: bool,
) -> Result<bool, ()> {
    if unbounded {
        projection.push("{\"kind\":\"unbounded\"}")?;
        return Ok(true);
    }
    let length = usize::try_from(cursor.read_i32()?).map_err(|_| ())?;
    let payload = cursor.read_exact(length)?;
    let component_limit = u64::try_from(MAX_JSON_INPUT_BYTES).unwrap_or(u64::MAX);
    let value = decode_value_at_depth(
        element_type,
        payload,
        component_limit,
        nesting_depth.saturating_add(1),
    )
    .map_err(|_| ())?;
    if matches!(value.as_ref(), ValueRef::Invalid { .. }) {
        return Err(());
    }
    if value.is_truncated() || matches!(value.as_ref(), ValueRef::Unknown { .. }) {
        return Ok(false);
    }
    projection.push(if inclusive {
        "{\"kind\":\"inclusive\",\"value\":"
    } else {
        "{\"kind\":\"exclusive\",\"value\":"
    })?;
    if !project_structured_value(&value, projection)? {
        return Ok(false);
    }
    projection.push("}")?;
    Ok(true)
}

#[derive(Clone, Copy)]
struct PostgresArrayDimension {
    length: usize,
    lower_bound: i32,
}

fn decode_array(
    element_type: &Type,
    raw: &[u8],
    limit: u64,
    nesting_depth: usize,
) -> Result<Option<OwnedValue>, ()> {
    let mut cursor = PostgresBinaryCursor::new(raw);
    let dimensions = usize::try_from(cursor.read_i32()?).map_err(|_| ())?;
    let has_null = cursor.read_i32()?;
    let element_oid = cursor.read_u32()?;
    if dimensions > MAX_POSTGRES_ARRAY_DIMENSIONS
        || !matches!(has_null, 0 | 1)
        || element_oid != element_type.oid()
    {
        return Err(());
    }
    let mut shape = Vec::with_capacity(dimensions);
    let mut elements = 1_usize;
    for _ in 0..dimensions {
        let length = usize::try_from(cursor.read_i32()?).map_err(|_| ())?;
        if length == 0 {
            return Err(());
        }
        elements = elements.checked_mul(length).ok_or(())?;
        if elements > MAX_POSTGRES_ARRAY_ELEMENTS {
            return Ok(None);
        }
        shape.push(PostgresArrayDimension {
            length,
            lower_bound: cursor.read_i32()?,
        });
    }
    let mut projection = BoundedJsonWriter::new(limit);
    projection.push("{\"$array\":{\"dimensions\":[")?;
    for (index, dimension) in shape.iter().enumerate() {
        if index != 0 {
            projection.push(",")?;
        }
        projection.push(&format!("[{},{}]", dimension.lower_bound, dimension.length))?;
    }
    projection.push("],\"values\":")?;
    let mut saw_null = false;
    let context = PostgresArrayProjectionContext {
        element_type,
        limit,
        nesting_depth,
    };
    if dimensions == 0 {
        projection.push("[]")?;
    } else if !project_array_values(
        &shape,
        0,
        &mut cursor,
        &mut projection,
        &mut saw_null,
        context,
    )? {
        return Ok(None);
    }
    if cursor.remaining() != 0 || (saw_null && has_null == 0) {
        return Err(());
    }
    projection.push("}}")?;
    projection.finish().map(Some).map_err(|_| ())
}

fn project_array_values(
    shape: &[PostgresArrayDimension],
    depth: usize,
    cursor: &mut PostgresBinaryCursor<'_>,
    projection: &mut BoundedJsonWriter,
    saw_null: &mut bool,
    context: PostgresArrayProjectionContext<'_>,
) -> Result<bool, ()> {
    projection.push("[")?;
    for index in 0..shape[depth].length {
        if index != 0 {
            projection.push(",")?;
        }
        if depth + 1 < shape.len() {
            if !project_array_values(shape, depth + 1, cursor, projection, saw_null, context)? {
                return Ok(false);
            }
            continue;
        }
        let length = cursor.read_i32()?;
        if length == -1 {
            *saw_null = true;
            projection.push("null")?;
            continue;
        }
        let length = usize::try_from(length).map_err(|_| ())?;
        let payload = cursor.read_exact(length)?;
        let value = decode_value_at_depth(
            context.element_type,
            payload,
            context.limit,
            context.nesting_depth.saturating_add(1),
        )
        .map_err(|_| ())?;
        if value.is_truncated() || !project_structured_value(&value, projection)? {
            return Ok(false);
        }
    }
    projection.push("]")?;
    Ok(true)
}

#[derive(Clone, Copy)]
struct PostgresArrayProjectionContext<'a> {
    element_type: &'a Type,
    limit: u64,
    nesting_depth: usize,
}

fn project_structured_value(
    value: &OwnedValue,
    projection: &mut BoundedJsonWriter,
) -> Result<bool, ()> {
    match value.as_ref() {
        ValueRef::Null => projection.push("null")?,
        ValueRef::Boolean(value) => projection.push(if value { "true" } else { "false" })?,
        ValueRef::Signed(value) => projection.push(&value.to_string())?,
        ValueRef::Unsigned(value) => projection.push(&value.to_string())?,
        ValueRef::Float64Bits(bits) => {
            projection.push(&format!("{{\"$float64_bits\":\"{bits:016x}\"}}"))?;
        }
        ValueRef::Decimal(value) => {
            projection.push("{\"$decimal\":")?;
            projection.push_json_string(value)?;
            projection.push("}")?;
        }
        ValueRef::Temporal { value, .. } | ValueRef::Text { value, .. } => {
            projection.push_json_string(value)?;
        }
        ValueRef::Structured { value, .. } => projection.push(value)?,
        ValueRef::Binary { value, .. } => {
            projection.push("{\"$binary\":\"")?;
            projection.push_hex(value)?;
            projection.push("\"}")?;
        }
        ValueRef::Invalid { .. } | ValueRef::Unknown { .. } => return Ok(false),
    }
    Ok(true)
}

struct PostgresBinaryCursor<'a> {
    remaining: &'a [u8],
}

impl<'a> PostgresBinaryCursor<'a> {
    const fn new(raw: &'a [u8]) -> Self {
        Self { remaining: raw }
    }

    const fn remaining(&self) -> usize {
        self.remaining.len()
    }

    fn read_u8(&mut self) -> Result<u8, ()> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_i32(&mut self) -> Result<i32, ()> {
        Ok(i32::from_be_bytes(
            self.read_exact(4)?.try_into().map_err(|_| ())?,
        ))
    }

    fn read_u32(&mut self) -> Result<u32, ()> {
        Ok(u32::from_be_bytes(
            self.read_exact(4)?.try_into().map_err(|_| ())?,
        ))
    }

    fn read_exact(&mut self, length: usize) -> Result<&'a [u8], ()> {
        let (value, remaining) = self.remaining.split_at_checked(length).ok_or(())?;
        self.remaining = remaining;
        Ok(value)
    }
}

const POSTGRES_UNIX_EPOCH_DAYS: i64 = 10_957;
const MICROS_PER_DAY: i64 = 86_400_000_000;

fn decode_temporal(type_: &Type, raw: &[u8], limit: u64) -> Result<OwnedValue, PostgresError> {
    let canonical = match *type_ {
        Type::DATE if raw.len() == 4 => {
            match i32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]) {
                i32::MAX => "infinity".to_owned(),
                i32::MIN => "-infinity".to_owned(),
                days => format_date_from_unix_days(i64::from(days) + POSTGRES_UNIX_EPOCH_DAYS),
            }
        }
        Type::TIME if raw.len() == 8 => {
            let micros = i64::from_be_bytes([
                raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
            ]);
            if !(0..=MICROS_PER_DAY).contains(&micros) {
                return bounded_raw(type_, raw, limit, true);
            }
            format_time(micros)
        }
        Type::TIMESTAMP | Type::TIMESTAMPTZ if raw.len() == 8 => {
            let micros = i64::from_be_bytes([
                raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
            ]);
            match micros {
                i64::MAX => "infinity".to_owned(),
                i64::MIN => "-infinity".to_owned(),
                micros => {
                    let days = micros.div_euclid(MICROS_PER_DAY) + POSTGRES_UNIX_EPOCH_DAYS;
                    let time = micros.rem_euclid(MICROS_PER_DAY);
                    let suffix = if *type_ == Type::TIMESTAMPTZ { "Z" } else { "" };
                    format!(
                        "{}T{}{}",
                        format_date_from_unix_days(days),
                        format_time(time),
                        suffix
                    )
                }
            }
        }
        Type::TIMETZ if raw.len() == 12 => {
            let micros = i64::from_be_bytes([
                raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
            ]);
            let seconds_west = i32::from_be_bytes([raw[8], raw[9], raw[10], raw[11]]);
            if !(0..=MICROS_PER_DAY).contains(&micros) {
                return bounded_raw(type_, raw, limit, true);
            }
            let Some(offset) = format_utc_offset(-i64::from(seconds_west)) else {
                return bounded_raw(type_, raw, limit, true);
            };
            format!("{}{}", format_time(micros), offset)
        }
        Type::INTERVAL if raw.len() == 16 => {
            let micros = i64::from_be_bytes([
                raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
            ]);
            let days = i32::from_be_bytes([raw[8], raw[9], raw[10], raw[11]]);
            let months = i32::from_be_bytes([raw[12], raw[13], raw[14], raw[15]]);
            match (micros, days, months) {
                (i64::MAX, i32::MAX, i32::MAX) => "infinity".to_owned(),
                (i64::MIN, i32::MIN, i32::MIN) => "-infinity".to_owned(),
                (micros, days, months) => {
                    format!("P{months}M{days}DT{}S", format_interval_seconds(micros))
                }
            }
        }
        Type::DATE
        | Type::TIME
        | Type::TIMETZ
        | Type::TIMESTAMP
        | Type::TIMESTAMPTZ
        | Type::INTERVAL => {
            return bounded_raw(type_, raw, limit, true);
        }
        _ => return bounded_raw(type_, raw, limit, false),
    };
    bounded_temporal(&canonical, limit)
}

fn bounded_temporal(canonical: &str, limit: u64) -> Result<OwnedValue, PostgresError> {
    let stored_len = usize::try_from(limit)
        .unwrap_or(usize::MAX)
        .min(canonical.len());
    let stored = BoundedText::copy_from_str(
        &canonical[..stored_len],
        ByteLimit::new(u64::try_from(stored_len).unwrap_or(u64::MAX)),
    )
    .map_err(|_| PostgresError::Protocol)?;
    OwnedValue::temporal(stored, truncation(stored_len, canonical.len()))
        .map_err(|_| PostgresError::Protocol)
}

fn format_time(micros: i64) -> String {
    let hours = micros / 3_600_000_000;
    let minutes = micros / 60_000_000 % 60;
    let seconds = micros / 1_000_000 % 60;
    let fraction = micros % 1_000_000;
    if fraction == 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{hours:02}:{minutes:02}:{seconds:02}.{fraction:06}")
    }
}

fn format_utc_offset(seconds_east: i64) -> Option<String> {
    const MAX_OFFSET_SECONDS: i64 = 15 * 3_600 + 59 * 60 + 59;
    let absolute = seconds_east.abs();
    if absolute > MAX_OFFSET_SECONDS {
        return None;
    }
    let sign = if seconds_east < 0 { '-' } else { '+' };
    let hours = absolute / 3_600;
    let minutes = absolute / 60 % 60;
    let seconds = absolute % 60;
    Some(if seconds == 0 {
        format!("{sign}{hours:02}:{minutes:02}")
    } else {
        format!("{sign}{hours:02}:{minutes:02}:{seconds:02}")
    })
}

fn format_interval_seconds(micros: i64) -> String {
    let absolute = micros.unsigned_abs();
    let seconds = absolute / 1_000_000;
    let fraction = absolute % 1_000_000;
    let sign = if micros < 0 { "-" } else { "" };
    if fraction == 0 {
        format!("{sign}{seconds}")
    } else {
        format!("{sign}{seconds}.{fraction:06}")
    }
}

fn decode_uuid(type_: &Type, raw: &[u8], limit: u64) -> Result<OwnedValue, PostgresError> {
    let bytes: [u8; 16] = match raw.try_into() {
        Ok(bytes) => bytes,
        Err(_) => return bounded_raw(type_, raw, limit, true),
    };
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut canonical = String::with_capacity(36);
    for (index, byte) in bytes.into_iter().enumerate() {
        if matches!(index, 4 | 6 | 8 | 10) {
            canonical.push('-');
        }
        canonical.push(char::from(HEX[usize::from(byte >> 4)]));
        canonical.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    let stored_len = usize::try_from(limit)
        .unwrap_or(usize::MAX)
        .min(canonical.len());
    let stored = BoundedText::copy_from_str(
        &canonical[..stored_len],
        ByteLimit::new(u64::try_from(stored_len).unwrap_or(u64::MAX)),
    )
    .map_err(|_| PostgresError::Protocol)?;
    OwnedValue::text(stored, truncation(stored_len, canonical.len()))
        .map_err(|_| PostgresError::Protocol)
}

fn decode_numeric(type_: &Type, raw: &[u8], limit: u64) -> Result<OwnedValue, PostgresError> {
    let Some(header) = NumericHeader::parse(raw) else {
        return bounded_raw(type_, raw, limit, true);
    };
    let projection = match header.project(limit) {
        Ok(Some(projection)) => projection,
        Ok(None) => return bounded_raw(type_, raw, limit, false),
        Err(()) => return bounded_raw(type_, raw, limit, true),
    };
    let decimal = BoundedText::from_string(projection, ByteLimit::new(limit))
        .map_err(|_| PostgresError::Protocol)?;
    Ok(OwnedValue::decimal(decimal))
}

const NUMERIC_POSITIVE: u16 = 0x0000;
const NUMERIC_NEGATIVE: u16 = 0x4000;
const NUMERIC_NAN: u16 = 0xC000;
const NUMERIC_POSITIVE_INFINITY: u16 = 0xD000;
const NUMERIC_NEGATIVE_INFINITY: u16 = 0xF000;
const NUMERIC_SCALE_MASK: u16 = 0x3FFF;

struct NumericHeader<'a> {
    weight: i16,
    sign: u16,
    scale: u16,
    digits: &'a [u8],
}

impl<'a> NumericHeader<'a> {
    fn parse(raw: &'a [u8]) -> Option<Self> {
        let header: [u8; 8] = raw.get(..8)?.try_into().ok()?;
        let digit_count = usize::from(u16::from_be_bytes([header[0], header[1]]));
        let expected = 8_usize.checked_add(digit_count.checked_mul(2)?)?;
        if raw.len() != expected {
            return None;
        }
        let sign = u16::from_be_bytes([header[4], header[5]]);
        let scale = u16::from_be_bytes([header[6], header[7]]);
        if !matches!(
            sign,
            NUMERIC_POSITIVE
                | NUMERIC_NEGATIVE
                | NUMERIC_NAN
                | NUMERIC_POSITIVE_INFINITY
                | NUMERIC_NEGATIVE_INFINITY
        ) || scale & !NUMERIC_SCALE_MASK != 0
            || raw[8..]
                .chunks_exact(2)
                .any(|digit| u16::from_be_bytes([digit[0], digit[1]]) >= 10_000)
        {
            return None;
        }
        Some(Self {
            weight: i16::from_be_bytes([header[2], header[3]]),
            sign,
            scale,
            digits: &raw[8..],
        })
    }

    fn project(&self, limit: u64) -> Result<Option<String>, ()> {
        let special = match self.sign {
            NUMERIC_NAN => Some("NaN"),
            NUMERIC_POSITIVE_INFINITY => Some("Infinity"),
            NUMERIC_NEGATIVE_INFINITY => Some("-Infinity"),
            _ => None,
        };
        if let Some(special) = special {
            return Ok((u64::try_from(special.len()).unwrap_or(u64::MAX) <= limit)
                .then(|| special.to_owned()));
        }

        let limit = usize::try_from(limit).unwrap_or(usize::MAX);
        let mut output = String::with_capacity(limit.min(256));
        let first_nonzero = self
            .digits
            .chunks_exact(2)
            .position(|digit| u16::from_be_bytes([digit[0], digit[1]]) != 0);
        let effective_weight = first_nonzero
            .map(|index| i32::from(self.weight) - i32::try_from(index).unwrap_or(i32::MAX));
        if let Some(weight) = effective_weight.filter(|weight| *weight >= 0) {
            let first = self.digit_at_exponent(weight).ok_or(())?.to_string();
            if !push_decimal(&mut output, &first, limit) {
                return Ok(None);
            }
            for exponent in (0..weight).rev() {
                if !push_decimal_group(
                    &mut output,
                    self.digit_at_exponent(exponent).unwrap_or(0),
                    4,
                    limit,
                ) {
                    return Ok(None);
                }
            }
        } else if !push_decimal(&mut output, "0", limit) {
            return Ok(None);
        }

        if self.scale != 0 {
            if !push_decimal(&mut output, ".", limit) {
                return Ok(None);
            }
            let groups = usize::from(self.scale).div_ceil(4);
            for group in 1..=groups {
                let digits = if group == groups && !self.scale.is_multiple_of(4) {
                    usize::from(self.scale % 4)
                } else {
                    4
                };
                let exponent = -i32::try_from(group).unwrap_or(i32::MAX);
                if !push_decimal_group(
                    &mut output,
                    self.digit_at_exponent(exponent).unwrap_or(0),
                    digits,
                    limit,
                ) {
                    return Ok(None);
                }
            }
        }
        if self.sign == NUMERIC_NEGATIVE
            && output
                .bytes()
                .any(|byte| byte.is_ascii_digit() && byte != b'0')
        {
            if output.len() >= limit {
                return Ok(None);
            }
            output.insert(0, '-');
        }
        Ok(Some(output))
    }

    fn digit_at_exponent(&self, exponent: i32) -> Option<u16> {
        let index = i32::from(self.weight).checked_sub(exponent)?;
        let index = usize::try_from(index).ok()?;
        let offset = index.checked_mul(2)?;
        let digit = self.digits.get(offset..offset + 2)?;
        Some(u16::from_be_bytes([digit[0], digit[1]]))
    }
}

fn push_decimal(output: &mut String, value: &str, limit: usize) -> bool {
    if output
        .len()
        .checked_add(value.len())
        .is_none_or(|length| length > limit)
    {
        return false;
    }
    output.push_str(value);
    true
}

fn push_decimal_group(output: &mut String, digit: u16, digits: usize, limit: usize) -> bool {
    let bytes = [
        b'0' + u8::try_from(digit / 1_000).unwrap_or(0),
        b'0' + u8::try_from((digit / 100) % 10).unwrap_or(0),
        b'0' + u8::try_from((digit / 10) % 10).unwrap_or(0),
        b'0' + u8::try_from(digit % 10).unwrap_or(0),
    ];
    let group = std::str::from_utf8(&bytes[..digits]).unwrap_or("");
    push_decimal(output, group, limit)
}

fn decode_json(type_: &Type, raw: &[u8], limit: u64) -> Result<OwnedValue, PostgresError> {
    let payload = if *type_ == Type::JSONB {
        match raw.split_first() {
            Some((&1, payload)) => payload,
            _ => return bounded_raw(type_, raw, limit, true),
        }
    } else {
        raw
    };
    if payload.len() > MAX_JSON_INPUT_BYTES {
        return bounded_raw(type_, raw, limit, false);
    }
    let parsed = match serde_json::from_slice::<serde_json::Value>(payload) {
        Ok(parsed) => parsed,
        Err(_) => return bounded_raw(type_, raw, limit, true),
    };
    let mut projection = BoundedJsonWriter::new(limit);
    serde_json::to_writer(&mut projection, &parsed).map_err(|_| PostgresError::Protocol)?;
    projection.finish()
}

const MAX_JSON_INPUT_BYTES: usize = 8 * 1_024 * 1_024;

struct BoundedJsonWriter {
    stored: Vec<u8>,
    original_byte_len: u64,
    limit: u64,
}

impl BoundedJsonWriter {
    fn new(limit: u64) -> Self {
        Self {
            stored: Vec::new(),
            original_byte_len: 0,
            limit,
        }
    }

    fn push(&mut self, value: &str) -> Result<(), ()> {
        self.write_all(value.as_bytes()).map_err(|_| ())
    }

    fn push_json_string(&mut self, value: &str) -> Result<(), ()> {
        serde_json::to_writer(self, value).map_err(|_| ())
    }

    fn push_hex(&mut self, value: &[u8]) -> Result<(), ()> {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        for byte in value {
            self.write_all(&[HEX[usize::from(byte >> 4)], HEX[usize::from(byte & 0x0f)]])
                .map_err(|_| ())?;
        }
        Ok(())
    }

    fn finish(mut self) -> Result<OwnedValue, PostgresError> {
        while std::str::from_utf8(&self.stored).is_err() {
            self.stored.pop();
        }
        let stored = String::from_utf8(self.stored).map_err(|_| PostgresError::Protocol)?;
        let stored_len = u64::try_from(stored.len()).unwrap_or(u64::MAX);
        let truncation = if stored_len == self.original_byte_len {
            Truncation::Complete
        } else {
            Truncation::Truncated {
                original_byte_len: Some(self.original_byte_len),
            }
        };
        OwnedValue::structured(
            BoundedText::from_string(stored, ByteLimit::new(self.limit))
                .map_err(|_| PostgresError::Protocol)?,
            truncation,
        )
        .map_err(|_| PostgresError::Protocol)
    }
}

impl Write for BoundedJsonWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.original_byte_len = self
            .original_byte_len
            .saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
        let remaining = self
            .limit
            .saturating_sub(u64::try_from(self.stored.len()).unwrap_or(u64::MAX));
        let take = usize::try_from(remaining)
            .unwrap_or(usize::MAX)
            .min(bytes.len());
        self.stored.extend_from_slice(&bytes[..take]);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
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

    #[test]
    fn json_projection_is_compact_sorted_and_bounded() {
        let value = decode_value(
            &Type::JSON,
            r#"{ "z": "éé", "a": [1, true] }"#.as_bytes(),
            18,
        )
        .unwrap();
        assert!(matches!(
            value.as_ref(),
            tablerock_core::ValueRef::Structured {
                value: "{\"a\":[1,true],\"z\":",
                truncation: Truncation::Truncated {
                    original_byte_len: Some(25)
                }
            }
        ));
    }

    #[test]
    fn jsonb_requires_version_one_and_valid_json() {
        let valid = decode_value(&Type::JSONB, b"\x01{\"a\":1}", 64).unwrap();
        assert_eq!(valid.kind(), tablerock_core::ValueKind::Structured);
        for raw in [&b"\x02{\"a\":1}"[..], &b"\x01{"[..]] {
            let invalid = decode_value(&Type::JSONB, raw, 64).unwrap();
            assert_eq!(invalid.kind(), tablerock_core::ValueKind::Invalid);
            assert_eq!(invalid.engine_type().unwrap().name(), "jsonb");
        }
    }

    #[test]
    fn json_projection_preserves_arbitrary_precision_numbers() {
        let value = decode_value(
            &Type::JSON,
            b"12345678901234567890.12345678901234567890",
            64,
        )
        .unwrap();
        assert!(matches!(
            value.as_ref(),
            tablerock_core::ValueRef::Structured {
                value: "12345678901234567890.12345678901234567890",
                truncation: Truncation::Complete
            }
        ));
    }

    #[test]
    fn oversized_json_stays_bounded_unknown_without_dom_allocation() {
        let raw = vec![b' '; MAX_JSON_INPUT_BYTES + 1];
        let value = decode_value(&Type::JSON, &raw, 8).unwrap();
        assert_eq!(value.kind(), tablerock_core::ValueKind::Unknown);
        assert_eq!(value.encoded_byte_len(), 8);
        match value.as_ref() {
            tablerock_core::ValueRef::Unknown {
                payload: b"        ",
                truncation:
                    Truncation::Truncated {
                        original_byte_len: Some(original),
                    },
                ..
            } => assert_eq!(original, (MAX_JSON_INPUT_BYTES + 1) as u64),
            _ => panic!("expected bounded oversized JSON"),
        }
    }

    #[test]
    fn numeric_projection_preserves_scale_sign_and_special_values() {
        for (raw, expected) in [
            (
                numeric_raw(0, NUMERIC_POSITIVE, 3, &[123, 4_500]),
                "123.450",
            ),
            (
                numeric_raw(-1, NUMERIC_NEGATIVE, 7, &[12, 3_000]),
                "-0.0012300",
            ),
            (numeric_raw(0, NUMERIC_POSITIVE, 3, &[]), "0.000"),
            (numeric_raw(-1, NUMERIC_NEGATIVE, 2, &[12]), "0.00"),
            (numeric_raw(0, NUMERIC_NAN, 0, &[]), "NaN"),
            (
                numeric_raw(0, NUMERIC_POSITIVE_INFINITY, 32, &[]),
                "Infinity",
            ),
            (
                numeric_raw(0, NUMERIC_NEGATIVE_INFINITY, 0, &[]),
                "-Infinity",
            ),
        ] {
            let value = decode_value(&Type::NUMERIC, &raw, 64).unwrap();
            assert!(matches!(
                value.as_ref(),
                tablerock_core::ValueRef::Decimal(value) if value == expected
            ));
        }
    }

    #[test]
    fn numeric_projection_bounds_and_rejects_malformed_wire_values() {
        let raw = numeric_raw(1, NUMERIC_POSITIVE, 4, &[1, 2, 3]);
        let bounded = decode_value(&Type::NUMERIC, &raw, 4).unwrap();
        assert_eq!(bounded.kind(), tablerock_core::ValueKind::Unknown);
        assert_eq!(bounded.encoded_byte_len(), 4);

        for raw in [
            numeric_raw(0, 0x8000, 0, &[]),
            numeric_raw(0, NUMERIC_POSITIVE, 0, &[10_000]),
            numeric_raw(0, NUMERIC_POSITIVE, 0x4000, &[]),
            vec![0; 7],
        ] {
            let invalid = decode_value(&Type::NUMERIC, &raw, 64).unwrap();
            assert_eq!(invalid.kind(), tablerock_core::ValueKind::Invalid);
            assert_eq!(invalid.engine_type().unwrap().name(), "numeric");
        }
    }

    #[test]
    fn uuid_projection_is_canonical_bounded_and_malformed_safe() {
        let raw = [
            0x12, 0x3e, 0x45, 0x67, 0xe8, 0x9b, 0x12, 0xd3, 0xa4, 0x56, 0x42, 0x66, 0x14, 0x17,
            0x40, 0x00,
        ];
        let complete = decode_value(&Type::UUID, &raw, 36).unwrap();
        assert!(matches!(
            complete.as_ref(),
            tablerock_core::ValueRef::Text {
                value: "123e4567-e89b-12d3-a456-426614174000",
                truncation: Truncation::Complete
            }
        ));

        let bounded = decode_value(&Type::UUID, &raw, 9).unwrap();
        assert!(matches!(
            bounded.as_ref(),
            tablerock_core::ValueRef::Text {
                value: "123e4567-",
                truncation: Truncation::Truncated {
                    original_byte_len: Some(36)
                }
            }
        ));

        for raw in [&raw[..15], &[0_u8; 17][..]] {
            let invalid = decode_value(&Type::UUID, raw, 8).unwrap();
            assert_eq!(invalid.kind(), tablerock_core::ValueKind::Invalid);
            assert_eq!(invalid.engine_type().unwrap().name(), "uuid");
        }
    }

    #[test]
    fn temporal_projection_handles_epochs_boundaries_and_infinity() {
        for (type_, raw, expected) in [
            (Type::DATE, 0_i32.to_be_bytes().to_vec(), "2000-01-01"),
            (
                Type::DATE,
                (-10_957_i32).to_be_bytes().to_vec(),
                "1970-01-01",
            ),
            (
                Type::TIME,
                MICROS_PER_DAY.to_be_bytes().to_vec(),
                "24:00:00",
            ),
            (
                Type::TIMESTAMP,
                (-1_i64).to_be_bytes().to_vec(),
                "1999-12-31T23:59:59.999999",
            ),
            (
                Type::TIMESTAMPTZ,
                0_i64.to_be_bytes().to_vec(),
                "2000-01-01T00:00:00Z",
            ),
            (Type::DATE, i32::MAX.to_be_bytes().to_vec(), "infinity"),
            (
                Type::TIMESTAMPTZ,
                i64::MIN.to_be_bytes().to_vec(),
                "-infinity",
            ),
        ] {
            let value = decode_value(&type_, &raw, 64).unwrap();
            assert!(matches!(
                value.as_ref(),
                tablerock_core::ValueRef::Temporal {
                    value,
                    truncation: Truncation::Complete
                } if value == expected
            ));
        }

        assert_eq!(format_date_from_unix_days(-719_528), "0000-01-01");
        assert_eq!(format_date_from_unix_days(2_933_262), "+10000-12-31");

        let mut timetz = Vec::new();
        timetz.extend_from_slice(&45_296_123_456_i64.to_be_bytes());
        timetz.extend_from_slice(&(-23_400_i32).to_be_bytes());
        let zoned = decode_value(&Type::TIMETZ, &timetz, 64).unwrap();
        assert!(matches!(
            zoned.as_ref(),
            tablerock_core::ValueRef::Temporal {
                value: "12:34:56.123456+06:30",
                truncation: Truncation::Complete
            }
        ));

        let mixed =
            decode_value(&Type::INTERVAL, &interval_raw(-14_706_123_456, -3, 14), 64).unwrap();
        assert!(matches!(
            mixed.as_ref(),
            tablerock_core::ValueRef::Temporal {
                value: "P14M-3DT-14706.123456S",
                truncation: Truncation::Complete
            }
        ));

        for (raw, expected) in [
            (interval_raw(i64::MAX, i32::MAX, i32::MAX), "infinity"),
            (interval_raw(i64::MIN, i32::MIN, i32::MIN), "-infinity"),
        ] {
            let value = decode_value(&Type::INTERVAL, &raw, 64).unwrap();
            assert!(matches!(
                value.as_ref(),
                tablerock_core::ValueRef::Temporal {
                    value,
                    truncation: Truncation::Complete
                } if value == expected
            ));
        }
    }

    #[test]
    fn temporal_projection_is_bounded_and_malformed_safe() {
        let bounded = decode_value(&Type::DATE, &0_i32.to_be_bytes(), 7).unwrap();
        assert!(matches!(
            bounded.as_ref(),
            tablerock_core::ValueRef::Temporal {
                value: "2000-01",
                truncation: Truncation::Truncated {
                    original_byte_len: Some(10)
                }
            }
        ));

        for (type_, raw) in [
            (Type::DATE, vec![0; 3]),
            (Type::TIME, (-1_i64).to_be_bytes().to_vec()),
            (Type::TIME, (MICROS_PER_DAY + 1).to_be_bytes().to_vec()),
            (Type::TIMETZ, vec![0; 11]),
            (Type::TIMETZ, {
                let mut raw = Vec::new();
                raw.extend_from_slice(&0_i64.to_be_bytes());
                raw.extend_from_slice(&(-57_600_i32).to_be_bytes());
                raw
            }),
            (Type::INTERVAL, vec![0; 15]),
            (Type::TIMESTAMP, vec![0; 7]),
        ] {
            let invalid = decode_value(&type_, &raw, 8).unwrap();
            assert_eq!(invalid.kind(), tablerock_core::ValueKind::Invalid);
            assert_eq!(invalid.engine_type().unwrap().name(), type_.name());
        }
    }

    #[test]
    fn array_projection_preserves_shape_lower_bounds_and_nulls() {
        let empty = array_raw(&[], false, &[]);
        let value = decode_value(&Type::INT4_ARRAY, &empty, 128).unwrap();
        assert!(matches!(
            value.as_ref(),
            tablerock_core::ValueRef::Structured {
                value: "{\"$array\":{\"dimensions\":[],\"values\":[]}}",
                truncation: Truncation::Complete
            }
        ));

        let vector = array_raw(&[(3, 0)], true, &[Some(7), None, Some(-2)]);
        let value = decode_value(&Type::INT4_ARRAY, &vector, 256).unwrap();
        assert!(matches!(
            value.as_ref(),
            tablerock_core::ValueRef::Structured {
                value: "{\"$array\":{\"dimensions\":[[0,3]],\"values\":[7,null,-2]}}",
                truncation: Truncation::Complete
            }
        ));

        let matrix = array_raw(
            &[(2, 1), (2, -1)],
            false,
            &[Some(1), Some(2), Some(3), Some(4)],
        );
        let value = decode_value(&Type::INT4_ARRAY, &matrix, 256).unwrap();
        assert!(matches!(
            value.as_ref(),
            tablerock_core::ValueRef::Structured {
                value: "{\"$array\":{\"dimensions\":[[1,2],[-1,2]],\"values\":[[1,2],[3,4]]}}",
                truncation: Truncation::Complete
            }
        ));
    }

    #[test]
    fn range_projection_preserves_bound_kinds_and_values() {
        let empty = decode_value(&Type::INT4_RANGE, &[RANGE_EMPTY], 128).unwrap();
        assert!(matches!(
            empty.as_ref(),
            ValueRef::Structured {
                value: "{\"$range\":{\"empty\":true}}",
                truncation: Truncation::Complete
            }
        ));

        let bounded = range_raw(
            RANGE_LOWER_INCLUSIVE,
            Some(&1_i32.to_be_bytes()),
            Some(&5_i32.to_be_bytes()),
        );
        let value = decode_value(&Type::INT4_RANGE, &bounded, 256).unwrap();
        assert!(matches!(
            value.as_ref(),
            ValueRef::Structured {
                value: "{\"$range\":{\"empty\":false,\"lower\":{\"kind\":\"inclusive\",\"value\":1},\"upper\":{\"kind\":\"exclusive\",\"value\":5}}}",
                truncation: Truncation::Complete
            }
        ));

        let upper = 42_i64.to_be_bytes();
        let unbounded = range_raw(
            RANGE_LOWER_UNBOUNDED | RANGE_UPPER_INCLUSIVE,
            None,
            Some(&upper),
        );
        let value = decode_value(&Type::INT8_RANGE, &unbounded, 256).unwrap();
        assert!(matches!(
            value.as_ref(),
            ValueRef::Structured {
                value: "{\"$range\":{\"empty\":false,\"lower\":{\"kind\":\"unbounded\"},\"upper\":{\"kind\":\"inclusive\",\"value\":42}}}",
                truncation: Truncation::Complete
            }
        ));
    }

    #[test]
    fn range_projection_bounds_output_and_rejects_malformed_wire() {
        let raw = range_raw(
            RANGE_LOWER_INCLUSIVE,
            Some(&1_i32.to_be_bytes()),
            Some(&5_i32.to_be_bytes()),
        );
        let bounded = decode_value(&Type::INT4_RANGE, &raw, 8).unwrap();
        assert!(matches!(
            bounded.as_ref(),
            ValueRef::Structured {
                value: "{\"$range",
                truncation: Truncation::Truncated {
                    original_byte_len: Some(original)
                }
            } if original > 8
        ));

        let timestamp = range_raw(
            RANGE_LOWER_INCLUSIVE,
            Some(&0_i64.to_be_bytes()),
            Some(&1_i64.to_be_bytes()),
        );
        let bounded = decode_value(&Type::TSTZ_RANGE, &timestamp, 8).unwrap();
        assert!(matches!(
            bounded.as_ref(),
            ValueRef::Structured {
                value: "{\"$range",
                truncation: Truncation::Truncated {
                    original_byte_len: Some(original)
                }
            } if original > 8
        ));

        let mut trailing = raw.clone();
        trailing.push(0);
        let malformed_subtype = range_raw(0, Some(&[0, 0, 0]), Some(&5_i32.to_be_bytes()));
        for malformed in [
            vec![],
            vec![RANGE_EMPTY, 0],
            vec![RANGE_EMPTY | RANGE_LOWER_INCLUSIVE],
            vec![0x80],
            vec![RANGE_LOWER_UNBOUNDED | RANGE_LOWER_INCLUSIVE],
            vec![0, 0, 0, 0, 4, 0],
            vec![0, 0xff, 0xff, 0xff, 0xff],
            range_raw(0, None, None),
            trailing,
            malformed_subtype,
        ] {
            let value = decode_value(&Type::INT4_RANGE, &malformed, 16).unwrap();
            assert_eq!(value.kind(), tablerock_core::ValueKind::Invalid);
            assert_eq!(value.engine_type().unwrap().name(), "int4range");
        }

        let unsupported_type = Type::new(
            "customrange".to_owned(),
            900_001,
            Kind::Range(Type::JSONPATH),
            "public".to_owned(),
        );
        let unsupported = range_raw(0, Some(b"$.a"), Some(b"$.z"));
        let value = decode_value(&unsupported_type, &unsupported, 64).unwrap();
        assert_eq!(value.kind(), tablerock_core::ValueKind::Unknown);
        assert_eq!(value.engine_type().unwrap().name(), "customrange");
    }

    #[test]
    fn multirange_projection_preserves_order_and_range_truth() {
        let empty = multirange_raw(&[]);
        let value = decode_value(&Type::INT4MULTI_RANGE, &empty, 128).unwrap();
        assert!(matches!(
            value.as_ref(),
            ValueRef::Structured {
                value: "{\"$multirange\":[]}",
                truncation: Truncation::Complete
            }
        ));

        let first = range_raw(
            RANGE_LOWER_INCLUSIVE,
            Some(&1_i32.to_be_bytes()),
            Some(&3_i32.to_be_bytes()),
        );
        let second = range_raw(
            RANGE_UPPER_UNBOUNDED | RANGE_LOWER_INCLUSIVE,
            Some(&10_i32.to_be_bytes()),
            None,
        );
        let raw = multirange_raw(&[&first, &second]);
        let value = decode_value(&Type::INT4MULTI_RANGE, &raw, 512).unwrap();
        assert!(matches!(
            value.as_ref(),
            ValueRef::Structured {
                value: "{\"$multirange\":[{\"$range\":{\"empty\":false,\"lower\":{\"kind\":\"inclusive\",\"value\":1},\"upper\":{\"kind\":\"exclusive\",\"value\":3}}},{\"$range\":{\"empty\":false,\"lower\":{\"kind\":\"inclusive\",\"value\":10},\"upper\":{\"kind\":\"unbounded\"}}}]}",
                truncation: Truncation::Complete
            }
        ));
    }

    #[test]
    fn multirange_projection_bounds_output_and_rejects_malformed_wire() {
        let range = range_raw(
            RANGE_LOWER_INCLUSIVE,
            Some(&1_i32.to_be_bytes()),
            Some(&3_i32.to_be_bytes()),
        );
        let raw = multirange_raw(&[&range]);
        let bounded = decode_value(&Type::INT4MULTI_RANGE, &raw, 8).unwrap();
        assert!(matches!(
            bounded.as_ref(),
            ValueRef::Structured {
                value: "{\"$multi",
                truncation: Truncation::Truncated {
                    original_byte_len: Some(original)
                }
            } if original > 8
        ));

        let mut trailing = multirange_raw(&[]);
        trailing.push(0);
        let malformed_range = multirange_raw(&[&[RANGE_EMPTY, 0]]);
        for malformed in [
            vec![],
            vec![0, 0, 0],
            vec![0, 0, 0, 1],
            vec![0, 0, 0, 1, 0, 0, 0, 4, RANGE_EMPTY],
            trailing,
            malformed_range,
        ] {
            let value = decode_value(&Type::INT4MULTI_RANGE, &malformed, 16).unwrap();
            assert_eq!(value.kind(), tablerock_core::ValueKind::Invalid);
            assert_eq!(value.engine_type().unwrap().name(), "int4multirange");
        }

        let excessive = (u32::try_from(MAX_POSTGRES_ARRAY_ELEMENTS).unwrap() + 1).to_be_bytes();
        let value = decode_value(&Type::INT4MULTI_RANGE, &excessive, 16).unwrap();
        assert_eq!(value.kind(), tablerock_core::ValueKind::Unknown);
    }

    #[test]
    fn composite_projection_preserves_named_and_anonymous_fields() {
        let named_type = Type::new(
            "probe_composite".to_owned(),
            900_002,
            Kind::Composite(vec![
                Field::new("id".to_owned(), Type::INT4),
                Field::new("label\"é".to_owned(), Type::TEXT),
                Field::new("absent".to_owned(), Type::TEXT),
            ]),
            "public".to_owned(),
        );
        let id = 7_i32.to_be_bytes();
        let raw = composite_raw(&[
            (Type::INT4.oid(), Some(id.as_slice())),
            (Type::TEXT.oid(), Some("é".as_bytes())),
            (Type::TEXT.oid(), None),
        ]);
        let value = decode_value(&named_type, &raw, 512).unwrap();
        assert!(matches!(
            value.as_ref(),
            ValueRef::Structured {
                value: "{\"$composite\":{\"fields\":[{\"name\":\"id\",\"oid\":23,\"type\":\"int4\",\"value\":7},{\"name\":\"label\\\"é\",\"oid\":25,\"type\":\"text\",\"value\":\"é\"},{\"name\":\"absent\",\"oid\":25,\"type\":\"text\",\"value\":null}]}}",
                truncation: Truncation::Complete
            }
        ));

        let raw = composite_raw(&[
            (Type::INT4.oid(), Some(id.as_slice())),
            (Type::TEXT.oid(), None),
        ]);
        let value = decode_value(&Type::RECORD, &raw, 512).unwrap();
        assert!(matches!(
            value.as_ref(),
            ValueRef::Structured {
                value: "{\"$composite\":{\"fields\":[{\"name\":null,\"oid\":23,\"type\":\"int4\",\"value\":7},{\"name\":null,\"oid\":25,\"type\":\"text\",\"value\":null}]}}",
                truncation: Truncation::Complete
            }
        ));
    }

    #[test]
    fn composite_projection_bounds_output_and_rejects_malformed_wire() {
        let id = 7_i32.to_be_bytes();
        let raw = composite_raw(&[(Type::INT4.oid(), Some(id.as_slice()))]);
        let bounded = decode_value(&Type::RECORD, &raw, 8).unwrap();
        assert!(matches!(
            bounded.as_ref(),
            ValueRef::Structured {
                value: "{\"$compo",
                truncation: Truncation::Truncated {
                    original_byte_len: Some(original)
                }
            } if original > 8
        ));

        let named_type = Type::new(
            "probe_composite".to_owned(),
            900_002,
            Kind::Composite(vec![Field::new("id".to_owned(), Type::INT4)]),
            "public".to_owned(),
        );
        let wrong_oid = composite_raw(&[(Type::TEXT.oid(), Some(b"7"))]);
        let malformed_value = composite_raw(&[(Type::INT4.oid(), Some(&[0, 0, 0]))]);
        let mut trailing = raw.clone();
        trailing.push(0);
        for malformed in [
            vec![],
            vec![0, 0, 0],
            composite_raw(&[]),
            wrong_oid,
            malformed_value,
            vec![0, 0, 0, 1, 0, 0, 0, 23, 0xff, 0xff, 0xff, 0xfe],
            trailing,
        ] {
            let value = decode_value(&named_type, &malformed, 32).unwrap();
            assert_eq!(value.kind(), tablerock_core::ValueKind::Invalid);
            assert_eq!(value.engine_type().unwrap().name(), "probe_composite");
        }

        let unsupported = composite_raw(&[(900_003, Some(b"opaque"))]);
        let value = decode_value(&Type::RECORD, &unsupported, 32).unwrap();
        assert_eq!(value.kind(), tablerock_core::ValueKind::Unknown);
        let excessive = (u32::try_from(MAX_POSTGRES_COMPOSITE_FIELDS).unwrap() + 1).to_be_bytes();
        let value = decode_value(&Type::RECORD, &excessive, 32).unwrap();
        assert_eq!(value.kind(), tablerock_core::ValueKind::Unknown);
        let value =
            decode_value_at_depth(&Type::RECORD, &raw, 32, MAX_POSTGRES_NESTING_DEPTH + 1).unwrap();
        assert_eq!(value.kind(), tablerock_core::ValueKind::Unknown);
    }

    #[test]
    fn domain_projection_reuses_underlying_semantics() {
        let integer_domain = domain_type("positive", 900_004, Type::INT4);
        let value = decode_value(&integer_domain, &7_i32.to_be_bytes(), 64).unwrap();
        assert!(matches!(value.as_ref(), ValueRef::Signed(7)));

        let nested_domain = domain_type("nested_positive", 900_005, integer_domain);
        let value = decode_value(&nested_domain, &8_i32.to_be_bytes(), 64).unwrap();
        assert!(matches!(value.as_ref(), ValueRef::Signed(8)));

        let array_domain = domain_type("ints", 900_006, Type::INT4_ARRAY);
        let raw = array_raw(&[(2, 1)], false, &[Some(1), Some(2)]);
        let value = decode_value(&array_domain, &raw, 256).unwrap();
        assert!(matches!(
            value.as_ref(),
            ValueRef::Structured {
                value: "{\"$array\":{\"dimensions\":[[1,2]],\"values\":[1,2]}}",
                truncation: Truncation::Complete
            }
        ));
    }

    #[test]
    fn domain_projection_preserves_outer_failure_identity_and_depth_bound() {
        let integer_domain = domain_type("positive", 900_004, Type::INT4);
        let invalid = decode_value(&integer_domain, &[0, 0, 0], 16).unwrap();
        assert_eq!(invalid.kind(), tablerock_core::ValueKind::Invalid);
        assert_eq!(invalid.engine_type().unwrap().name(), "positive");

        let unsupported_domain = domain_type("path_domain", 900_005, Type::JSONPATH);
        let unknown = decode_value(&unsupported_domain, b"$.a", 16).unwrap();
        assert_eq!(unknown.kind(), tablerock_core::ValueKind::Unknown);
        assert_eq!(unknown.engine_type().unwrap().name(), "path_domain");

        let bounded = decode_value(&integer_domain, &7_i32.to_be_bytes(), 4).unwrap();
        assert_eq!(bounded.kind(), tablerock_core::ValueKind::Unknown);
        assert_eq!(bounded.engine_type().unwrap().name(), "positive");

        let value = decode_value_at_depth(
            &integer_domain,
            &7_i32.to_be_bytes(),
            16,
            MAX_POSTGRES_NESTING_DEPTH,
        )
        .unwrap();
        assert_eq!(value.kind(), tablerock_core::ValueKind::Unknown);
        assert_eq!(value.engine_type().unwrap().name(), "positive");
    }

    #[test]
    fn enum_projection_validates_and_bounds_catalog_labels() {
        let enum_type = Type::new(
            "status".to_owned(),
            900_007,
            Kind::Enum(vec![
                "ready".to_owned(),
                "café".to_owned(),
                "blocked".to_owned(),
            ]),
            "public".to_owned(),
        );
        let value = decode_value(&enum_type, "café".as_bytes(), 16).unwrap();
        assert!(matches!(
            value.as_ref(),
            ValueRef::Text {
                value: "café",
                truncation: Truncation::Complete
            }
        ));

        let bounded = decode_value(&enum_type, "café".as_bytes(), 4).unwrap();
        assert!(matches!(
            bounded.as_ref(),
            ValueRef::Text {
                value: "caf",
                truncation: Truncation::Truncated {
                    original_byte_len: Some(5)
                }
            }
        ));

        for malformed in [b"unknown".as_slice(), &[0xff]] {
            let value = decode_value(&enum_type, malformed, 16).unwrap();
            assert_eq!(value.kind(), tablerock_core::ValueKind::Invalid);
            assert_eq!(value.engine_type().unwrap().name(), "status");
        }
    }

    #[test]
    fn array_projection_bounds_output_and_rejects_malformed_wire() {
        let raw = array_raw(&[(3, 1)], false, &[Some(1), Some(2), Some(3)]);
        let bounded = decode_value(&Type::INT4_ARRAY, &raw, 8).unwrap();
        assert!(matches!(
            bounded.as_ref(),
            tablerock_core::ValueRef::Structured {
                value: "{\"$array",
                truncation: Truncation::Truncated {
                    original_byte_len: Some(original)
                }
            } if original > 8
        ));

        let mut wrong_oid = raw.clone();
        wrong_oid[8..12].copy_from_slice(&Type::TEXT.oid().to_be_bytes());
        let mut trailing = raw.clone();
        trailing.push(0);
        let null_without_flag = array_raw(&[(1, 1)], false, &[None]);
        let mut invalid_null_flag = raw.clone();
        invalid_null_flag[4..8].copy_from_slice(&2_i32.to_be_bytes());
        let mut negative_dimension = raw.clone();
        negative_dimension[12..16].copy_from_slice(&(-1_i32).to_be_bytes());
        for malformed in [
            wrong_oid,
            trailing,
            null_without_flag,
            invalid_null_flag,
            negative_dimension,
            vec![0; 11],
        ] {
            let value = decode_value(&Type::INT4_ARRAY, &malformed, 16).unwrap();
            assert_eq!(value.kind(), tablerock_core::ValueKind::Invalid);
            assert_eq!(value.engine_type().unwrap().name(), "_int4");
        }

        let mut excessive = Vec::new();
        excessive.extend_from_slice(&2_i32.to_be_bytes());
        excessive.extend_from_slice(&0_i32.to_be_bytes());
        excessive.extend_from_slice(&Type::INT4.oid().to_be_bytes());
        for length in [1_001_i32, 1_000_i32] {
            excessive.extend_from_slice(&length.to_be_bytes());
            excessive.extend_from_slice(&1_i32.to_be_bytes());
        }
        let value = decode_value(&Type::INT4_ARRAY, &excessive, 16).unwrap();
        assert_eq!(value.kind(), tablerock_core::ValueKind::Unknown);
    }

    fn numeric_raw(weight: i16, sign: u16, scale: u16, digits: &[u16]) -> Vec<u8> {
        let mut raw = Vec::with_capacity(8 + digits.len() * 2);
        raw.extend_from_slice(&u16::try_from(digits.len()).unwrap().to_be_bytes());
        raw.extend_from_slice(&weight.to_be_bytes());
        raw.extend_from_slice(&sign.to_be_bytes());
        raw.extend_from_slice(&scale.to_be_bytes());
        for digit in digits {
            raw.extend_from_slice(&digit.to_be_bytes());
        }
        raw
    }

    fn interval_raw(microseconds: i64, days: i32, months: i32) -> Vec<u8> {
        let mut raw = Vec::with_capacity(16);
        raw.extend_from_slice(&microseconds.to_be_bytes());
        raw.extend_from_slice(&days.to_be_bytes());
        raw.extend_from_slice(&months.to_be_bytes());
        raw
    }

    fn array_raw(dimensions: &[(i32, i32)], has_null: bool, values: &[Option<i32>]) -> Vec<u8> {
        let mut raw = Vec::new();
        raw.extend_from_slice(&i32::try_from(dimensions.len()).unwrap().to_be_bytes());
        raw.extend_from_slice(&i32::from(has_null).to_be_bytes());
        raw.extend_from_slice(&Type::INT4.oid().to_be_bytes());
        for (length, lower_bound) in dimensions {
            raw.extend_from_slice(&length.to_be_bytes());
            raw.extend_from_slice(&lower_bound.to_be_bytes());
        }
        for value in values {
            match value {
                Some(value) => {
                    raw.extend_from_slice(&4_i32.to_be_bytes());
                    raw.extend_from_slice(&value.to_be_bytes());
                }
                None => raw.extend_from_slice(&(-1_i32).to_be_bytes()),
            }
        }
        raw
    }

    fn range_raw(flags: u8, lower: Option<&[u8]>, upper: Option<&[u8]>) -> Vec<u8> {
        let mut raw = vec![flags];
        for bound in [lower, upper].into_iter().flatten() {
            raw.extend_from_slice(&i32::try_from(bound.len()).unwrap().to_be_bytes());
            raw.extend_from_slice(bound);
        }
        raw
    }

    fn multirange_raw(ranges: &[&[u8]]) -> Vec<u8> {
        let mut raw = Vec::new();
        raw.extend_from_slice(&u32::try_from(ranges.len()).unwrap().to_be_bytes());
        for range in ranges {
            raw.extend_from_slice(&u32::try_from(range.len()).unwrap().to_be_bytes());
            raw.extend_from_slice(range);
        }
        raw
    }

    fn composite_raw(fields: &[(u32, Option<&[u8]>)]) -> Vec<u8> {
        let mut raw = Vec::new();
        raw.extend_from_slice(&u32::try_from(fields.len()).unwrap().to_be_bytes());
        for (oid, value) in fields {
            raw.extend_from_slice(&oid.to_be_bytes());
            match value {
                Some(value) => {
                    raw.extend_from_slice(&i32::try_from(value.len()).unwrap().to_be_bytes());
                    raw.extend_from_slice(value);
                }
                None => raw.extend_from_slice(&(-1_i32).to_be_bytes()),
            }
        }
        raw
    }

    fn domain_type(name: &str, oid: u32, underlying: Type) -> Type {
        Type::new(
            name.to_owned(),
            oid,
            Kind::Domain(underlying),
            "public".to_owned(),
        )
    }
}
