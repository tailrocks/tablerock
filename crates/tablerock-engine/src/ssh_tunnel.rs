//! SSH tunnel adapter below database clients (`russh`).
//!
//! Drivers receive only the established local endpoint (or a tunnelled stream).
//! Passwords never appear in Debug; no shell interpolation.

use std::{fmt, sync::Arc};

use russh::client::{self, AuthResult, Handle, Handler};
use russh::{Channel, ChannelStream};
use tokio::net::TcpListener;

/// Host-key verification policy for the tunnel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SshHostKeyPolicy {
    /// Accept any host key. **Local tests only** — never for production profiles.
    DangerousAcceptAnyForTests,
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

/// Opaque russh client handler (host-key policy applied).
pub struct ClientHandler {
    policy: SshHostKeyPolicy,
}

impl Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        match self.policy {
            SshHostKeyPolicy::DangerousAcceptAnyForTests => Ok(true),
        }
    }
}

/// Open SSH session with password auth (host-key policy enforced).
pub async fn connect_session(
    config: &SshTunnelConfig,
) -> Result<Handle<ClientHandler>, SshTunnelError> {
    let conf = Arc::new(client::Config::default());
    let handler = ClientHandler {
        policy: config.host_key_policy,
    };
    let mut handle = client::connect(
        conf,
        (config.bastion_host.as_str(), config.bastion_port),
        handler,
    )
    .await
    .map_err(|_| SshTunnelError::Connect)?;

    let auth = handle
        .authenticate_password(config.auth.username.as_str(), config.auth.password.as_str())
        .await
        .map_err(|_| SshTunnelError::Auth)?;
    match auth {
        AuthResult::Success => Ok(handle),
        AuthResult::Failure { .. } => Err(SshTunnelError::Auth),
    }
}

/// Open a direct-tcpip channel through an authenticated session.
pub async fn open_direct_tcpip(
    handle: &Handle<ClientHandler>,
    target_host: &str,
    target_port: u16,
) -> Result<Channel<client::Msg>, SshTunnelError> {
    handle
        .channel_open_direct_tcpip(
            target_host,
            u32::from(target_port),
            "127.0.0.1",
            0,
        )
        .await
        .map_err(|_| SshTunnelError::Channel)
}

/// Convert a channel into a bidirectional stream for driver use.
#[must_use]
pub fn channel_stream(channel: Channel<client::Msg>) -> ChannelStream<client::Msg> {
    channel.into_stream()
}

/// Local listener that accepts one connection and bridges it over direct-tcpip.
///
/// Returns the bound local port. A background task serves the first accepted
/// connection; drop the returned join handle / session to stop.
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
        let Ok((mut local, _)) = listener.accept().await else {
            return;
        };
        let Ok(channel) = open_direct_tcpip(&handle, &target_host, target_port).await else {
            return;
        };
        let mut remote = channel_stream(channel);
        let _ = tokio::io::copy_bidirectional(&mut local, &mut remote).await;
        // Keep handle alive until bridge ends.
        drop(handle);
    });

    Ok((local_port, join))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_auth_debug_redacts_secret() {
        let auth = SshPasswordAuth::new("tunnel", "super-secret");
        let debug = format!("{auth:?}");
        assert!(!debug.contains("super-secret"));
        assert!(debug.contains("<redacted>"));
    }
}
