//! Parse supported database URLs into a reviewable connection draft.
//!
//! Supported schemes (case-insensitive):
//! - `postgresql` / `postgres`
//! - `clickhouse` / `http` / `https` (ClickHouse HTTP; https ⇒ TLS)
//! - `redis` / `rediss` (rediss ⇒ TLS)
//!
//! Credentials stay in the draft only; Debug redacts password bytes.
//! Percent-decoding applied to userinfo and path segments.

use std::{error::Error, fmt};

use crate::Engine;

/// Maximum accepted URL length (reject before parse work).
pub const MAX_CONNECTION_URL_BYTES: usize = 4_096;

/// TLS intent derived from scheme or query parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectionUrlTls {
    /// No TLS requested by URL.
    Off,
    /// TLS required (scheme or sslmode).
    Required,
}

/// Reviewable draft fields from a URL (presentation maps into editor/draft).
#[derive(Clone, PartialEq, Eq)]
pub struct ConnectionUrlDraft {
    pub engine: Engine,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    /// Present only when the URL carried a password component.
    pub password: Option<String>,
    pub tls: ConnectionUrlTls,
}

impl fmt::Debug for ConnectionUrlDraft {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConnectionUrlDraft")
            .field("engine", &self.engine)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("database", &self.database)
            .field("username", &self.username)
            .field(
                "password",
                &self
                    .password
                    .as_ref()
                    .map(|p| format!("[{} bytes]", p.len())),
            )
            .field("tls", &self.tls)
            .finish()
    }
}

impl ConnectionUrlDraft {
    /// Operator-facing summary for confirm dialogs — never includes password text.
    #[must_use]
    pub fn safety_summary(&self) -> String {
        let engine = match self.engine {
            Engine::PostgreSql => "PostgreSQL",
            Engine::ClickHouse => "ClickHouse",
            Engine::Redis => "Redis",
        };
        let user = if self.username.is_empty() {
            "(none)"
        } else {
            self.username.as_str()
        };
        let secret = if self.password.is_some() {
            "present"
        } else {
            "absent"
        };
        let tls = match self.tls {
            ConnectionUrlTls::Off => "off",
            ConnectionUrlTls::Required => "required",
        };
        format!(
            "{engine} {host}:{port}/{db} user={user} password={secret} tls={tls}",
            host = self.host,
            port = self.port,
            db = self.database,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionUrlError {
    Empty,
    TooLarge {
        actual: usize,
        maximum: usize,
    },
    UnsupportedScheme {
        scheme: String,
    },
    MissingHost,
    InvalidHost,
    InvalidPort {
        value: String,
    },
    InvalidEncoding,
    Malformed,
    /// Control characters or other hostile bytes in the input.
    HostileInput,
}

impl fmt::Display for ConnectionUrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("connection URL is empty"),
            Self::TooLarge { actual, maximum } => {
                write!(f, "connection URL {actual} bytes exceeds max {maximum}")
            }
            Self::UnsupportedScheme { scheme } => {
                write!(f, "unsupported URL scheme '{scheme}'")
            }
            Self::MissingHost => f.write_str("connection URL is missing host"),
            Self::InvalidHost => f.write_str("connection URL host is invalid"),
            Self::InvalidPort { value } => write!(f, "invalid port '{value}'"),
            Self::InvalidEncoding => f.write_str("invalid percent-encoding in URL"),
            Self::Malformed => f.write_str("malformed connection URL"),
            Self::HostileInput => f.write_str("connection URL contains hostile input"),
        }
    }
}

impl Error for ConnectionUrlError {}

/// Parse a connection URL into a reviewable draft.
pub fn parse_connection_url(input: &str) -> Result<ConnectionUrlDraft, ConnectionUrlError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ConnectionUrlError::Empty);
    }
    if trimmed.len() > MAX_CONNECTION_URL_BYTES {
        return Err(ConnectionUrlError::TooLarge {
            actual: trimmed.len(),
            maximum: MAX_CONNECTION_URL_BYTES,
        });
    }
    // Hostile: C0 controls (except tab is still rejected), DEL, and NULs.
    if trimmed.bytes().any(|b| b < 0x20 || b == 0x7f) {
        return Err(ConnectionUrlError::HostileInput);
    }

    let (scheme, rest) = split_scheme(trimmed)?;
    let scheme_lower = scheme.to_ascii_lowercase();

    let (engine, default_port, scheme_tls) = match scheme_lower.as_str() {
        "postgresql" | "postgres" => (Engine::PostgreSql, 5432_u16, ConnectionUrlTls::Off),
        "clickhouse" | "http" => (Engine::ClickHouse, 8123, ConnectionUrlTls::Off),
        "https" => (Engine::ClickHouse, 8443, ConnectionUrlTls::Required),
        "redis" => (Engine::Redis, 6379, ConnectionUrlTls::Off),
        "rediss" => (Engine::Redis, 6379, ConnectionUrlTls::Required),
        // Explicit reject list for common deep-link attack schemes.
        "javascript" | "data" | "file" | "about" | "blob" | "vbscript" | "mailto" => {
            return Err(ConnectionUrlError::HostileInput);
        }
        other => {
            return Err(ConnectionUrlError::UnsupportedScheme {
                scheme: other.into(),
            });
        }
    };

    // Strip optional query for authority parse; keep for sslmode.
    let (authority_and_path, query) = match rest.split_once('?') {
        Some((a, q)) => (a, Some(q)),
        None => (rest, None),
    };

    // After scheme:// the remainder is user:pass@host:port/db
    let (userinfo, hostport_path) = match authority_and_path.split_once('@') {
        Some((ui, hp)) => (Some(ui), hp),
        None => (None, authority_and_path),
    };

    let (username, password) = match userinfo {
        Some(ui) => parse_userinfo(ui)?,
        None => (String::new(), None),
    };

    let (hostport, path) = match hostport_path.split_once('/') {
        Some((hp, p)) => (hp, Some(p)),
        None => (hostport_path, None),
    };

    if hostport.is_empty() {
        return Err(ConnectionUrlError::MissingHost);
    }

    // IPv6: [::1]:5432
    let (host, port) = if hostport.starts_with('[') {
        let end = hostport.find(']').ok_or(ConnectionUrlError::Malformed)?;
        let host = hostport[1..end].to_owned();
        if host.is_empty() {
            return Err(ConnectionUrlError::MissingHost);
        }
        let after = &hostport[end + 1..];
        let port = if let Some(p) = after.strip_prefix(':') {
            parse_port(p)?
        } else if after.is_empty() {
            default_port
        } else {
            return Err(ConnectionUrlError::Malformed);
        };
        (host, port)
    } else {
        match hostport.rsplit_once(':') {
            Some((h, p)) if !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) => {
                if h.is_empty() {
                    return Err(ConnectionUrlError::MissingHost);
                }
                (h.to_owned(), parse_port(p)?)
            }
            _ => (hostport.to_owned(), default_port),
        }
    };
    validate_host(&host)?;

    let database = match path {
        Some(p) if !p.is_empty() => {
            // First path segment only; strip trailing slash material.
            let seg = p.split('/').next().unwrap_or(p);
            percent_decode(seg)?
        }
        _ => match engine {
            Engine::PostgreSql => "postgres".into(),
            Engine::ClickHouse => "default".into(),
            Engine::Redis => "0".into(),
        },
    };

    let mut tls = scheme_tls;
    if let Some(q) = query {
        if query_requests_tls(q) {
            tls = ConnectionUrlTls::Required;
        }
        if query_disables_tls(q) {
            tls = ConnectionUrlTls::Off;
        }
    }

    Ok(ConnectionUrlDraft {
        engine,
        host,
        port,
        database,
        username,
        password,
        tls,
    })
}

fn split_scheme(input: &str) -> Result<(&str, &str), ConnectionUrlError> {
    let (scheme, rest) = input
        .split_once("://")
        .ok_or(ConnectionUrlError::Malformed)?;
    if scheme.is_empty() {
        return Err(ConnectionUrlError::Malformed);
    }
    Ok((scheme, rest))
}

fn parse_userinfo(ui: &str) -> Result<(String, Option<String>), ConnectionUrlError> {
    match ui.split_once(':') {
        Some((u, p)) => Ok((percent_decode(u)?, Some(percent_decode(p)?))),
        None => Ok((percent_decode(ui)?, None)),
    }
}

fn parse_port(raw: &str) -> Result<u16, ConnectionUrlError> {
    raw.parse::<u16>()
        .ok()
        .filter(|&p| p > 0)
        .ok_or_else(|| ConnectionUrlError::InvalidPort {
            value: raw.to_owned(),
        })
}

fn query_requests_tls(query: &str) -> bool {
    for pair in query.split('&') {
        let (k, v) = match pair.split_once('=') {
            Some((k, v)) => (k.to_ascii_lowercase(), v.to_ascii_lowercase()),
            None => (pair.to_ascii_lowercase(), String::new()),
        };
        if (k == "sslmode" || k == "ssl")
            && matches!(
                v.as_str(),
                "require" | "verify-ca" | "verify-full" | "true" | "1" | "yes"
            )
        {
            return true;
        }
        if k == "secure" && matches!(v.as_str(), "true" | "1" | "yes") {
            return true;
        }
    }
    false
}

fn query_disables_tls(query: &str) -> bool {
    for pair in query.split('&') {
        let (k, v) = match pair.split_once('=') {
            Some((k, v)) => (k.to_ascii_lowercase(), v.to_ascii_lowercase()),
            None => continue,
        };
        if k == "sslmode" && matches!(v.as_str(), "disable" | "allow" | "prefer") {
            // prefer still allows non-TLS first; treat as Off for draft default.
            return v == "disable";
        }
    }
    false
}

fn percent_decode(input: &str) -> Result<String, ConnectionUrlError> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                if i + 2 >= bytes.len() {
                    return Err(ConnectionUrlError::InvalidEncoding);
                }
                let h = hex_nibble(bytes[i + 1])?;
                let l = hex_nibble(bytes[i + 2])?;
                out.push((h << 4) | l);
                i += 3;
            }
            b'+' => {
                // form-encoding space; uncommon in DB URLs but accept.
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|_| ConnectionUrlError::InvalidEncoding)
}

fn hex_nibble(b: u8) -> Result<u8, ConnectionUrlError> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(ConnectionUrlError::InvalidEncoding),
    }
}

/// Host must be non-empty DNS/IPv4/IPv6-ish without spaces or path separators.
fn validate_host(host: &str) -> Result<(), ConnectionUrlError> {
    if host.is_empty() || host.len() > 253 {
        return Err(ConnectionUrlError::InvalidHost);
    }
    if host.contains([' ', '/', '\\', '?', '#', '@']) {
        return Err(ConnectionUrlError::InvalidHost);
    }
    if host.bytes().any(|b| b < 0x20 || b == 0x7f) {
        return Err(ConnectionUrlError::HostileInput);
    }
    // Reject clearly non-host forms used in deep-link abuse.
    if host.eq_ignore_ascii_case("localhost")
        || host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':'))
    {
        return Ok(());
    }
    // Allow broader unicode hostnames but reject shell metacharacters.
    if host.chars().any(|c| {
        matches!(
            c,
            ';' | '|' | '&' | '`' | '$' | '(' | ')' | '<' | '>' | '\n' | '\r'
        )
    }) {
        return Err(ConnectionUrlError::HostileInput);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_postgres_full_url() {
        let d = parse_connection_url(
            "postgresql://alice:s%3Bcret@db.example:6543/app_db?sslmode=require",
        )
        .unwrap();
        assert_eq!(d.engine, Engine::PostgreSql);
        assert_eq!(d.host, "db.example");
        assert_eq!(d.port, 6543);
        assert_eq!(d.database, "app_db");
        assert_eq!(d.username, "alice");
        assert_eq!(d.password.as_deref(), Some("s;cret"));
        assert_eq!(d.tls, ConnectionUrlTls::Required);
        let debug = format!("{d:?}");
        assert!(!debug.contains("s;cret"));
        assert!(debug.contains("password"));
    }

    #[test]
    fn parses_redis_and_rediss() {
        let plain = parse_connection_url("redis://:hunter2@127.0.0.1:6380/3").unwrap();
        assert_eq!(plain.engine, Engine::Redis);
        assert_eq!(plain.port, 6380);
        assert_eq!(plain.database, "3");
        assert_eq!(plain.username, "");
        assert_eq!(plain.password.as_deref(), Some("hunter2"));
        assert_eq!(plain.tls, ConnectionUrlTls::Off);

        let tls = parse_connection_url("rediss://u@host/0").unwrap();
        assert_eq!(tls.tls, ConnectionUrlTls::Required);
        assert_eq!(tls.username, "u");
        assert_eq!(tls.port, 6379);
    }

    #[test]
    fn parses_clickhouse_https() {
        let d = parse_connection_url("https://default@ch.example:8443/analytics").unwrap();
        assert_eq!(d.engine, Engine::ClickHouse);
        assert_eq!(d.port, 8443);
        assert_eq!(d.database, "analytics");
        assert_eq!(d.tls, ConnectionUrlTls::Required);
    }

    #[test]
    fn ipv6_host() {
        let d = parse_connection_url("postgres://[::1]:5433/postgres").unwrap();
        assert_eq!(d.host, "::1");
        assert_eq!(d.port, 5433);
    }

    #[test]
    fn rejects_unsupported_and_empty() {
        assert!(matches!(
            parse_connection_url(""),
            Err(ConnectionUrlError::Empty)
        ));
        assert!(matches!(
            parse_connection_url("mysql://h/db"),
            Err(ConnectionUrlError::UnsupportedScheme { .. })
        ));
        assert!(matches!(
            parse_connection_url("not-a-url"),
            Err(ConnectionUrlError::Malformed)
        ));
    }

    #[test]
    fn defaults_when_parts_missing() {
        let d = parse_connection_url("postgres://localhost").unwrap();
        assert_eq!(d.port, 5432);
        assert_eq!(d.database, "postgres");
        assert!(d.password.is_none());
        assert_eq!(d.tls, ConnectionUrlTls::Off);
    }

    #[test]
    fn oversized_rejected() {
        let big = format!("postgres://h/{}", "x".repeat(MAX_CONNECTION_URL_BYTES));
        assert!(matches!(
            parse_connection_url(&big),
            Err(ConnectionUrlError::TooLarge { .. })
        ));
    }

    #[test]
    fn hostile_schemes_and_controls_rejected() {
        assert!(matches!(
            parse_connection_url("javascript:alert(1)"),
            Err(ConnectionUrlError::HostileInput | ConnectionUrlError::Malformed)
        ));
        // Explicit scheme with ://
        assert!(matches!(
            parse_connection_url("javascript://evil"),
            Err(ConnectionUrlError::HostileInput)
        ));
        assert!(matches!(
            parse_connection_url("file:///etc/passwd"),
            Err(ConnectionUrlError::HostileInput)
        ));
        assert!(matches!(
            parse_connection_url("postgres://h\0ost/db"),
            Err(ConnectionUrlError::HostileInput)
        ));
        assert!(matches!(
            parse_connection_url("postgres://host;rm/db"),
            Err(ConnectionUrlError::HostileInput)
        ));
    }

    #[test]
    fn safety_summary_never_includes_password() {
        let d = parse_connection_url("postgres://u:s3cret@h:1/db").unwrap();
        let s = d.safety_summary();
        assert!(!s.contains("s3cret"));
        assert!(s.contains("password=present"));
        assert!(s.contains("PostgreSQL"));
        assert!(s.contains("h:1/db"));
    }
}
