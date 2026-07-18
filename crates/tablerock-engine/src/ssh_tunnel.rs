//! SSH tunnel adapter below database clients (`russh`).
//!
//! Drivers receive only the established local endpoint (or a tunnelled stream).
//! Passwords never appear in Debug; no shell interpolation.

use std::{fmt, path::PathBuf, sync::Arc};

use russh::client::{self, AuthResult, Handle, Handler};
use russh::keys::{self, PublicKey};
use russh::{Channel, ChannelStream};
use tokio::net::TcpListener;

/// Host-key verification policy for the tunnel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshHostKeyPolicy {
    /// Accept any host key. **Local tests only** — never for production profiles.
    DangerousAcceptAnyForTests,
    /// Fail closed against an OpenSSH `known_hosts` file (host/port from config).
    KnownHostsPath(PathBuf),
}

/// Password authentication material (redacted in Debug).
pub struct SshPasswordAuth {
    username: String,
    password: String,
}

impl SshPasswordAuth {
    #[must_use]
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }
}

impl fmt::Debug for SshPasswordAuth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SshPasswordAuth")
            .field("username", &self.username)
            .field("password", &"<redacted>")
            .finish()
    }
}

/// Bastion endpoint and auth for opening tunnels.
#[derive(Debug)]
pub struct SshTunnelConfig {
    pub bastion_host: String,
    pub bastion_port: u16,
    pub auth: SshPasswordAuth,
    pub host_key_policy: SshHostKeyPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SshTunnelError {
    Connect,
    Auth,
    HostKeyRejected,
    Channel,
    Bind,
    Forward,
}

impl fmt::Display for SshTunnelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Connect => "SSH bastion connection failed",
            Self::Auth => "SSH authentication failed",
            Self::HostKeyRejected => "SSH host key rejected",
            Self::Channel => "SSH direct-tcpip channel failed",
            Self::Bind => "local tunnel listener bind failed",
            Self::Forward => "local tunnel forward failed",
        })
    }
}

impl std::error::Error for SshTunnelError {}

fn map_connect_error(error: russh::Error) -> SshTunnelError {
    match error {
        russh::Error::UnknownKey => SshTunnelError::HostKeyRejected,
        _ => SshTunnelError::Connect,
    }
}

fn host_key_accepted(
    policy: &SshHostKeyPolicy,
    bastion_host: &str,
    bastion_port: u16,
    server_public_key: &PublicKey,
) -> bool {
    match policy {
        SshHostKeyPolicy::DangerousAcceptAnyForTests => true,
        SshHostKeyPolicy::KnownHostsPath(path) => {
            match keys::check_known_hosts_path(bastion_host, bastion_port, server_public_key, path)
            {
                Ok(true) => true,
                // Unknown key, changed key, or unreadable file → reject.
                Ok(false) | Err(_) => false,
            }
        }
    }
}

/// Opaque russh client handler (host-key policy applied).
pub struct ClientHandler {
    policy: SshHostKeyPolicy,
    bastion_host: String,
    bastion_port: u16,
    presented: Arc<std::sync::Mutex<Option<PublicKey>>>,
}

impl Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        let accept = host_key_accepted(
            &self.policy,
            &self.bastion_host,
            self.bastion_port,
            server_public_key,
        );
        if accept {
            if let Ok(mut guard) = self.presented.lock() {
                *guard = Some(server_public_key.clone());
            }
        }
        Ok(accept)
    }
}

/// Open SSH session with password auth (host-key policy enforced).
pub async fn connect_session(
    config: &SshTunnelConfig,
) -> Result<Handle<ClientHandler>, SshTunnelError> {
    let (handle, _) = connect_session_capture_host_key(config).await?;
    Ok(handle)
}

/// Open SSH session and return the host key that passed policy.
///
/// On `KnownHostsPath`, the key is the one already trusted. On
/// `DangerousAcceptAnyForTests`, the key is the live bastion key (for learning).
pub async fn connect_session_capture_host_key(
    config: &SshTunnelConfig,
) -> Result<(Handle<ClientHandler>, PublicKey), SshTunnelError> {
    let conf = Arc::new(client::Config::default());
    let presented = Arc::new(std::sync::Mutex::new(None::<PublicKey>));
    let handler = ClientHandler {
        policy: config.host_key_policy.clone(),
        bastion_host: config.bastion_host.clone(),
        bastion_port: config.bastion_port,
        presented: Arc::clone(&presented),
    };
    let mut handle = client::connect(
        conf,
        (config.bastion_host.as_str(), config.bastion_port),
        handler,
    )
    .await
    .map_err(map_connect_error)?;

    let auth = handle
        .authenticate_password(config.auth.username.as_str(), config.auth.password.as_str())
        .await
        .map_err(|_| SshTunnelError::Auth)?;
    match auth {
        AuthResult::Success => {
            let key = presented
                .lock()
                .ok()
                .and_then(|g| g.clone())
                .ok_or(SshTunnelError::HostKeyRejected)?;
            Ok((handle, key))
        }
        AuthResult::Failure { .. } => Err(SshTunnelError::Auth),
    }
}

/// Record a host key into an OpenSSH known_hosts file.
pub fn learn_host_key(
    host: &str,
    port: u16,
    public_key: &PublicKey,
    path: impl AsRef<std::path::Path>,
) -> Result<(), SshTunnelError> {
    keys::known_hosts::learn_known_hosts_path(host, port, public_key, path)
        .map_err(|_| SshTunnelError::Connect)
}

/// Open a direct-tcpip channel through an authenticated session.
pub async fn open_direct_tcpip(
    handle: &Handle<ClientHandler>,
    target_host: &str,
    target_port: u16,
) -> Result<Channel<client::Msg>, SshTunnelError> {
    handle
        .channel_open_direct_tcpip(target_host, u32::from(target_port), "127.0.0.1", 0)
        .await
        .map_err(|_| SshTunnelError::Channel)
}

/// Convert a channel into a bidirectional stream for driver use.
#[must_use]
pub fn channel_stream(channel: Channel<client::Msg>) -> ChannelStream<client::Msg> {
    channel.into_stream()
}

/// Live local forward: drivers connect to `127.0.0.1:local_port` only.
///
/// Dropping this value aborts the accept loop and closes the SSH session.
pub struct LocalForwardTunnel {
    local_port: u16,
    join: tokio::task::JoinHandle<()>,
}

impl LocalForwardTunnel {
    #[must_use]
    pub const fn local_host() -> &'static str {
        "127.0.0.1"
    }

    #[must_use]
    pub const fn local_port(&self) -> u16 {
        self.local_port
    }
}

impl Drop for LocalForwardTunnel {
    fn drop(&mut self) {
        self.join.abort();
    }
}

impl fmt::Debug for LocalForwardTunnel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalForwardTunnel")
            .field("local_host", &Self::local_host())
            .field("local_port", &self.local_port)
            .finish_non_exhaustive()
    }
}

/// Bind `127.0.0.1:0`, open SSH session, and accept multiple TCP bridges.
pub async fn open_local_forward_tunnel(
    config: &SshTunnelConfig,
    target_host: impl Into<String>,
    target_port: u16,
) -> Result<LocalForwardTunnel, SshTunnelError> {
    let handle = connect_session(config).await?;
    let target_host = target_host.into();
    spawn_local_forward(handle, target_host, target_port)
        .await
        .map(|(local_port, join)| LocalForwardTunnel { local_port, join })
}

/// Local listener that accepts connections and bridges each over direct-tcpip.
///
/// Returns the bound local port. Drop the join handle (or abort it) to stop.
pub async fn spawn_local_forward(
    handle: Handle<ClientHandler>,
    target_host: String,
    target_port: u16,
) -> Result<(u16, tokio::task::JoinHandle<()>), SshTunnelError> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|_| SshTunnelError::Bind)?;
    let local_port = listener
        .local_addr()
        .map_err(|_| SshTunnelError::Bind)?
        .port();

    let join = tokio::spawn(async move {
        loop {
            let Ok((mut local, _)) = listener.accept().await else {
                break;
            };
            let Ok(channel) = open_direct_tcpip(&handle, &target_host, target_port).await else {
                break;
            };
            let mut remote = channel_stream(channel);
            // Concurrent bridges: spawn per accept so multi-statement drivers work.
            tokio::spawn(async move {
                let _ = tokio::io::copy_bidirectional(&mut local, &mut remote).await;
            });
        }
        drop(handle);
    });

    Ok((local_port, join))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn password_auth_debug_redacts_secret() {
        let auth = SshPasswordAuth::new("tunnel", "super-secret");
        let debug = format!("{auth:?}");
        assert!(!debug.contains("super-secret"));
        assert!(debug.contains("<redacted>"));
    }

    #[test]
    fn known_hosts_path_policy_debug_has_no_secrets() {
        let policy = SshHostKeyPolicy::KnownHostsPath(PathBuf::from("/tmp/known_hosts"));
        let debug = format!("{policy:?}");
        assert!(debug.contains("KnownHostsPath"));
    }

    #[test]
    fn empty_known_hosts_rejects_unknown_key() {
        let dir = std::env::temp_dir().join(format!("tablerock-kh-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("known_hosts");
        let _ = std::fs::File::create(&path).unwrap();

        let key = keys::parse_public_key_base64(
            "AAAAC3NzaC1lZDI1NTE5AAAAIJdD7y3aLq454yWBdwLWbieU1ebz9/cu7/QEXn9OIeZJ",
        )
        .unwrap();
        let ok = keys::check_known_hosts_path("127.0.0.1", 2222, &key, &path).unwrap();
        assert!(!ok, "empty known_hosts must not accept arbitrary keys");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn learn_then_check_round_trip() {
        let dir = std::env::temp_dir().join(format!("tablerock-kh-learn-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("known_hosts");
        let key = keys::parse_public_key_base64(
            "AAAAC3NzaC1lZDI1NTE5AAAAIJdD7y3aLq454yWBdwLWbieU1ebz9/cu7/QEXn9OIeZJ",
        )
        .unwrap();
        learn_host_key("127.0.0.1", 2222, &key, &path).unwrap();
        assert!(keys::check_known_hosts_path("127.0.0.1", 2222, &key, &path).unwrap());

        let other = keys::parse_public_key_base64(
            "AAAAC3NzaC1lZDI1NTE5AAAAILIG2T/B0l0gaqj3puu510tu9N1OkQ4znY3LYuEm5zCF",
        )
        .unwrap();
        let changed = keys::check_known_hosts_path("127.0.0.1", 2222, &other, &path);
        assert!(changed.is_err() || matches!(changed, Ok(false)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn known_hosts_file_format_matches_openssh() {
        let dir = std::env::temp_dir().join(format!("tablerock-kh-fmt-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("known_hosts");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(
                f,
                "[localhost]:13265 ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJdD7y3aLq454yWBdwLWbieU1ebz9/cu7/QEXn9OIeZJ"
            )
            .unwrap();
        }
        let key = keys::parse_public_key_base64(
            "AAAAC3NzaC1lZDI1NTE5AAAAIJdD7y3aLq454yWBdwLWbieU1ebz9/cu7/QEXn9OIeZJ",
        )
        .unwrap();
        assert!(keys::check_known_hosts_path("localhost", 13265, &key, &path).unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
