//! Real SSH bastion proofs: password auth, known_hosts, local-forward to
//! PostgreSQL / ClickHouse / Redis drivers (drivers remain SSH-unaware).
//!
//! Bastion is alpine+openssh with AllowTcpForwarding (linuxserver image
//! hard-disables TCP forwarding).

use std::path::PathBuf;

use tablerock_core::{BoundedText, ByteLimit};
use tablerock_engine::{
    ClickHouseCompression, ClickHouseConnectConfig, ClickHouseSession, ClickHouseTlsMode,
    LocalForwardTunnel, PostgresConnectConfig, PostgresSession, PostgresTlsMode,
    RedisConnectConfig, RedisConnectionSecurity, RedisProtocol, RedisSession, RedisTlsMode,
    SshAgentAuth, SshAuthMaterial, SshHostKeyPolicy, SshPasswordAuth, SshPublicKeyAuth,
    SshTunnelConfig, SshTunnelError, channel_stream, connect_session,
    connect_session_capture_host_key, learn_host_key, open_direct_tcpip, open_local_forward_tunnel,
    spawn_local_forward,
};
use testcontainers::{
    GenericImage, ImageExt,
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Alpine sshd entrypoint: password auth + TCP forwarding for tunnel tests.
const SSHD_BOOTSTRAP: &str = r#"
set -eu
apk add --no-cache openssh openssh-server >/dev/null
ssh-keygen -A >/dev/null
echo 'root:tunnel-pass' | chpasswd
cat >/etc/ssh/sshd_config <<'CFG'
Port 22
ListenAddress 0.0.0.0
PasswordAuthentication yes
PermitRootLogin yes
AllowTcpForwarding yes
GatewayPorts no
UsePAM no
Subsystem sftp /usr/lib/ssh/sftp-server
CFG
exec /usr/sbin/sshd -D -e
"#;

fn unique_tag() -> String {
    format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    )
}

async fn start_bastion_on_network(
    network: &str,
) -> (testcontainers::ContainerAsync<GenericImage>, u16) {
    let ssh = GenericImage::new("alpine", "3.21")
        .with_exposed_port(22.tcp())
        .with_wait_for(WaitFor::message_on_stderr(
            "Server listening on 0.0.0.0 port 22",
        ))
        .with_entrypoint("sh")
        .with_cmd(["-c", SSHD_BOOTSTRAP])
        .with_network(network)
        .start()
        .await
        .expect("alpine sshd bastion");
    let ssh_port = ssh.get_host_port_ipv4(22.tcp()).await.unwrap();
    (ssh, ssh_port)
}

fn accept_any_config(ssh_port: u16) -> SshTunnelConfig {
    SshTunnelConfig {
        bastion_host: "127.0.0.1".into(),
        bastion_port: ssh_port,
        auth: SshAuthMaterial::Password(SshPasswordAuth::new("root", "tunnel-pass")),
        host_key_policy: SshHostKeyPolicy::DangerousAcceptAnyForTests,
    }
}

/// Unencrypted OpenSSH ed25519 private key for bastion pubkey tests only.
const TEST_CLIENT_PRIVATE: &str = "-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
QyNTUxOQAAACCCsY3emDPC9Lvg3RpbTSDhT+d6SkRIDPhJbcCT0Pj2CAAAAKhL435IS+N+
SAAAAAtzc2gtZWQyNTUxOQAAACCCsY3emDPC9Lvg3RpbTSDhT+d6SkRIDPhJbcCT0Pj2CA
AAAEC8+/gN0GtMxo4cf9QP/TjWaFF3aDOGp9YqQ8CVl9mwMYKxjd6YM8L0u+DdGltNIOFP
53pKREgM+EltwJPQ+PYIAAAAImRvbmJlYXZlQEFsZXhleXMtTWFjQm9vay1Qcm8ubG9jYW
wBAgM=
-----END OPENSSH PRIVATE KEY-----
";

const TEST_CLIENT_PUBLIC: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIIKxjd6YM8L0u+DdGltNIOFP53pKREgM+EltwJPQ+PYI test@tablerock";

/// Bastion that accepts only the given public key (password auth off).
fn pubkey_sshd_bootstrap(authorized_public_key: &str) -> String {
    format!(
        r#"
set -eu
apk add --no-cache openssh openssh-server >/dev/null
ssh-keygen -A >/dev/null
mkdir -p /root/.ssh
chmod 700 /root/.ssh
printf '%s\n' '{pub}' > /root/.ssh/authorized_keys
chmod 600 /root/.ssh/authorized_keys
cat >/etc/ssh/sshd_config <<'CFG'
Port 22
ListenAddress 0.0.0.0
PasswordAuthentication no
PubkeyAuthentication yes
PermitRootLogin prohibit-password
AllowTcpForwarding yes
GatewayPorts no
UsePAM no
AuthorizedKeysFile .ssh/authorized_keys
Subsystem sftp /usr/lib/ssh/sftp-server
CFG
exec /usr/sbin/sshd -D -e
"#,
        pub = authorized_public_key
    )
}

/// Encrypted aes256-ctr OpenSSH key; passphrase `test-pass-phrase`.
const TEST_CLIENT_ENCRYPTED: &str = "-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAACmFlczI1Ni1jdHIAAAAGYmNyeXB0AAAAGAAAABBONFrUJM
IqwobiDgim6S+oAAAAGAAAAAEAAAAzAAAAC3NzaC1lZDI1NTE5AAAAIHHw5gseHBmFD4Fj
Bt//7cH6sWnFVekyGEm7PeF6ADHtAAAAoBaUP2fvUaHrxI4SdOIb3QMGjhxuqJKyAcL92C
52c0Hbf/YcY9SvUttZ7KNvIgtEAVXUa0afrEK20RNo0gsbrBaHnbfTf4oUD7JNerQjmvIY
IVnTpRrUT/0otbJ9Rvhk/0J/Qecd1XlPC6mVtFeLiRv/vOzXcJTsL/219lIP58PEQXLUvx
C/h2ADG+GuOY1seMXSQeOkWcDlPhdQ0QU8eeA=
-----END OPENSSH PRIVATE KEY-----
";

const TEST_CLIENT_ENCRYPTED_PUBLIC: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHHw5gseHBmFD4FjBt//7cH6sWnFVekyGEm7PeF6ADHt tablerock-test";

async fn open_tunnel_retry(
    config: &SshTunnelConfig,
    target: &str,
    target_port: u16,
) -> LocalForwardTunnel {
    let mut tunnel = None;
    for _ in 0..40 {
        match open_local_forward_tunnel(config, target, target_port).await {
            Ok(t) => {
                tunnel = Some(t);
                break;
            }
            Err(_) => tokio::time::sleep(std::time::Duration::from_millis(500)).await,
        }
    }
    tunnel.expect("local forward tunnel")
}

fn bt(s: &str) -> BoundedText {
    BoundedText::copy_from_str(s, ByteLimit::new(253)).unwrap()
}

async fn start_bastion_and_pg() -> (
    testcontainers::ContainerAsync<GenericImage>,
    testcontainers::ContainerAsync<GenericImage>,
    String,
    u16,
) {
    let tag = unique_tag();
    let network = format!("tablerock-ssh-{tag}");
    let pg_name = format!("tablerock-pg-{tag}");

    let postgres = GenericImage::new("postgres", "18.4-alpine")
        .with_exposed_port(5432.tcp())
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_HOST_AUTH_METHOD", "trust")
        .with_network(network.as_str())
        .with_container_name(pg_name.as_str())
        .start()
        .await
        .expect("postgres container");

    let (ssh, ssh_port) = start_bastion_on_network(network.as_str()).await;
    (postgres, ssh, pg_name, ssh_port)
}

#[tokio::test]
async fn password_auth_and_direct_tcpip_to_postgres() {
    let (_postgres, _ssh, pg_name, ssh_port) = start_bastion_and_pg().await;

    let config = accept_any_config(ssh_port);

    let mut handle = None;
    for _ in 0..40 {
        match connect_session(&config).await {
            Ok(h) => {
                handle = Some(h);
                break;
            }
            Err(_) => tokio::time::sleep(std::time::Duration::from_millis(500)).await,
        }
    }
    let handle = handle.expect("SSH password auth to bastion");

    let channel = open_direct_tcpip(&handle, &pg_name, 5432)
        .await
        .expect("direct-tcpip channel to postgres on shared network");
    let mut stream = channel_stream(channel);

    let _ = stream.write_all(b"\x00\x00\x00\x08\x04\xd2\x16\x2f").await;
    let mut buf = [0_u8; 1];
    let n = tokio::time::timeout(std::time::Duration::from_secs(5), stream.read(&mut buf))
        .await
        .expect("read timeout through tunnel")
        .expect("read through tunnel");
    assert!(n > 0, "postgres should answer SSLRequest (N or S)");
    assert!(
        buf[0] == b'N' || buf[0] == b'S',
        "unexpected postgres response byte {}",
        buf[0]
    );

    let handle2 = connect_session(&config).await.expect("second auth");
    let (local_port, join) = spawn_local_forward(handle2, pg_name, 5432)
        .await
        .expect("local forward bind");
    assert!(local_port > 0);

    let mut local = tokio::net::TcpStream::connect(("127.0.0.1", local_port))
        .await
        .expect("connect local forward");
    local
        .write_all(b"\x00\x00\x00\x08\x04\xd2\x16\x2f")
        .await
        .expect("write SSLRequest via local forward");
    let mut lbuf = [0_u8; 1];
    let ln = tokio::time::timeout(std::time::Duration::from_secs(5), local.read(&mut lbuf))
        .await
        .expect("local forward read timeout")
        .expect("local forward read");
    assert!(ln > 0);
    assert!(lbuf[0] == b'N' || lbuf[0] == b'S');

    join.abort();
}

#[tokio::test]
async fn known_hosts_fail_closed_and_accept_learned_key() {
    let (_postgres, _ssh, _pg_name, ssh_port) = start_bastion_and_pg().await;

    let dir = std::env::temp_dir().join(format!("tablerock-ssh-kh-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let known_hosts = dir.join("known_hosts");
    let _ = std::fs::File::create(&known_hosts).unwrap();

    let empty_cfg = SshTunnelConfig {
        bastion_host: "127.0.0.1".into(),
        bastion_port: ssh_port,
        auth: SshAuthMaterial::Password(SshPasswordAuth::new("root", "tunnel-pass")),
        host_key_policy: SshHostKeyPolicy::KnownHostsPath(known_hosts.clone()),
    };

    // Wait for bastion readiness via accept-any first.
    let mut live_key = None;
    for _ in 0..40 {
        let accept_any = accept_any_config(ssh_port);
        match connect_session_capture_host_key(&accept_any).await {
            Ok((_h, key)) => {
                live_key = Some(key);
                break;
            }
            Err(_) => tokio::time::sleep(std::time::Duration::from_millis(500)).await,
        }
    }
    let live_key = live_key.expect("capture live bastion host key");

    // Empty known_hosts → HostKeyRejected (fail closed).
    let reject = connect_session(&empty_cfg).await.err();
    assert_eq!(
        reject,
        Some(SshTunnelError::HostKeyRejected),
        "empty known_hosts must reject"
    );

    learn_host_key("127.0.0.1", ssh_port, &live_key, &known_hosts).expect("learn host key");

    let trusted = SshTunnelConfig {
        bastion_host: "127.0.0.1".into(),
        bastion_port: ssh_port,
        auth: SshAuthMaterial::Password(SshPasswordAuth::new("root", "tunnel-pass")),
        host_key_policy: SshHostKeyPolicy::KnownHostsPath(PathBuf::from(&known_hosts)),
    };
    connect_session(&trusted)
        .await
        .expect("known host key must authenticate");

    // Wrong key recorded for same host:port → reject.
    let wrong_path = dir.join("wrong_hosts");
    std::fs::write(
        &wrong_path,
        format!(
            "[127.0.0.1]:{ssh_port} ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILIG2T/B0l0gaqj3puu510tu9N1OkQ4znY3LYuEm5zCF\n"
        ),
    )
    .unwrap();
    let wrong_cfg = SshTunnelConfig {
        bastion_host: "127.0.0.1".into(),
        bastion_port: ssh_port,
        auth: SshAuthMaterial::Password(SshPasswordAuth::new("root", "tunnel-pass")),
        host_key_policy: SshHostKeyPolicy::KnownHostsPath(wrong_path),
    };
    let changed = connect_session(&wrong_cfg).await.err();
    assert_eq!(
        changed,
        Some(SshTunnelError::HostKeyRejected),
        "changed host key must reject"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn postgres_driver_connects_through_local_forward_only() {
    let (_postgres, _ssh, pg_name, ssh_port) = start_bastion_and_pg().await;
    let config = accept_any_config(ssh_port);
    let tunnel = open_tunnel_retry(&config, pg_name.as_str(), 5432).await;

    let host = bt(LocalForwardTunnel::local_host());
    let database = BoundedText::copy_from_str("postgres", ByteLimit::new(128)).unwrap();
    let user = BoundedText::copy_from_str("postgres", ByteLimit::new(128)).unwrap();
    let pg = PostgresConnectConfig::new(
        host,
        tunnel.local_port(),
        database,
        user,
        PostgresTlsMode::Disabled,
    );
    let session = PostgresSession::connect(&pg)
        .await
        .expect("PostgresSession via SSH local forward");
    session.health_check().await.expect("health through tunnel");
    let describe = session
        .describe_server()
        .await
        .expect("describe_server through tunnel");
    assert!(!describe.identity().is_empty());
    session.shutdown().await.ok();
    drop(tunnel);
}

#[tokio::test]
async fn clickhouse_driver_connects_through_local_forward_only() {
    let tag = unique_tag();
    let network = format!("tablerock-ssh-ch-{tag}");
    let ch_name = format!("tablerock-ch-{tag}");
    let image =
        "26.3.17.4-jammy@sha256:158dcce6f6fdc59309650aad6b79484abf4eed07d4e0bdba31d732e64b5a25fb";
    let _ch = GenericImage::new("clickhouse", image)
        .with_exposed_port(8123.tcp())
        .with_env_var("CLICKHOUSE_SKIP_USER_SETUP", "1")
        .with_network(network.as_str())
        .with_container_name(ch_name.as_str())
        .start()
        .await
        .expect("clickhouse container");
    let (_ssh, ssh_port) = start_bastion_on_network(network.as_str()).await;
    let config = accept_any_config(ssh_port);
    let tunnel = open_tunnel_retry(&config, ch_name.as_str(), 8123).await;

    let session = ClickHouseSession::connect(&ClickHouseConnectConfig::new(
        bt(LocalForwardTunnel::local_host()),
        tunnel.local_port(),
        BoundedText::copy_from_str("default", ByteLimit::new(128)).unwrap(),
        BoundedText::copy_from_str("default", ByteLimit::new(128)).unwrap(),
        ClickHouseTlsMode::Disable,
        ClickHouseCompression::None,
    ));
    let described = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        loop {
            match session.describe_server().await {
                Ok(d) => break d,
                Err(_) => tokio::time::sleep(std::time::Duration::from_millis(100)).await,
            }
        }
    })
    .await
    .expect("CH describe timeout through tunnel");
    assert!(!described.identity().is_empty());
    drop(tunnel);
}

async fn wait_connect(config: &SshTunnelConfig) -> bool {
    for _ in 0..40 {
        match connect_session(config).await {
            Ok(_) => return true,
            Err(_) => tokio::time::sleep(std::time::Duration::from_millis(500)).await,
        }
    }
    false
}

#[tokio::test]
async fn public_key_auth_to_bastion() {
    let tag = unique_tag();
    let network = format!("tablerock-ssh-pubkey-{tag}");
    let bootstrap = pubkey_sshd_bootstrap(TEST_CLIENT_PUBLIC);
    let ssh = GenericImage::new("alpine", "3.21")
        .with_exposed_port(22.tcp())
        .with_wait_for(WaitFor::message_on_stderr(
            "Server listening on 0.0.0.0 port 22",
        ))
        .with_entrypoint("sh")
        .with_cmd(["-c", bootstrap.as_str()])
        .with_network(network.as_str())
        .start()
        .await
        .expect("pubkey bastion");
    let ssh_port = ssh.get_host_port_ipv4(22.tcp()).await.unwrap();

    let key_auth = SshPublicKeyAuth::from_openssh_private_key("root", TEST_CLIENT_PRIVATE)
        .expect("parse client key");
    let config = SshTunnelConfig {
        bastion_host: "127.0.0.1".into(),
        bastion_port: ssh_port,
        auth: SshAuthMaterial::PublicKey(key_auth),
        host_key_policy: SshHostKeyPolicy::DangerousAcceptAnyForTests,
    };
    assert!(
        wait_connect(&config).await,
        "public-key auth to bastion must succeed"
    );

    // Password auth must fail on this bastion (password disabled).
    let password_cfg = SshTunnelConfig {
        bastion_host: "127.0.0.1".into(),
        bastion_port: ssh_port,
        auth: SshAuthMaterial::Password(SshPasswordAuth::new("root", "tunnel-pass")),
        host_key_policy: SshHostKeyPolicy::DangerousAcceptAnyForTests,
    };
    let password_result = connect_session(&password_cfg).await.err();
    assert_eq!(
        password_result,
        Some(SshTunnelError::Auth),
        "password auth must fail when bastion disables passwords"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn agent_auth_to_bastion() {
    use std::{
        pin::Pin,
        sync::Arc,
        task::{Context, Poll},
    };

    use futures_util::Stream;
    use russh::keys::agent::client::AgentClient;
    use russh::keys::agent::server::{self, Agent};
    use russh::keys::{PrivateKey, decode_secret_key};
    use tokio::net::{UnixListener, UnixStream};

    #[derive(Clone)]
    struct AllowAll;
    impl Agent for AllowAll {
        fn confirm(
            self,
            _: Arc<PrivateKey>,
        ) -> Box<dyn std::future::Future<Output = (Self, bool)> + Send + Unpin> {
            Box::new(std::future::ready((self, true)))
        }
    }

    /// Unpin accept stream for `russh` agent `serve`.
    struct AcceptStream {
        listener: UnixListener,
    }
    impl Stream for AcceptStream {
        type Item = std::io::Result<UnixStream>;
        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            match self.listener.poll_accept(cx) {
                Poll::Ready(Ok((stream, _))) => Poll::Ready(Some(Ok(stream))),
                Poll::Ready(Err(error)) => Poll::Ready(Some(Err(error))),
                Poll::Pending => Poll::Pending,
            }
        }
    }

    let tag = unique_tag();
    let sock = std::env::temp_dir().join(format!("tablerock-agent-{tag}.sock"));
    let _ = std::fs::remove_file(&sock);
    let listener = UnixListener::bind(&sock).expect("bind agent socket");
    let agent_task = tokio::spawn(async move {
        let _ = server::serve(AcceptStream { listener }, AllowAll).await;
    });
    // Give serve a beat to open the path.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let private = decode_secret_key(TEST_CLIENT_PRIVATE, None).expect("decode client key");
    // Seed agent with identity (never leaves process except via sign requests).
    {
        let stream = UnixStream::connect(&sock)
            .await
            .expect("connect agent for add_identity");
        let mut agent = AgentClient::connect(stream);
        agent
            .add_identity(&private, &[])
            .await
            .expect("add identity to agent");
    }

    let network = format!("tablerock-ssh-agent-{tag}");
    let bootstrap = pubkey_sshd_bootstrap(TEST_CLIENT_PUBLIC);
    let ssh = GenericImage::new("alpine", "3.21")
        .with_exposed_port(22.tcp())
        .with_wait_for(WaitFor::message_on_stderr(
            "Server listening on 0.0.0.0 port 22",
        ))
        .with_entrypoint("sh")
        .with_cmd(["-c", bootstrap.as_str()])
        .with_network(network.as_str())
        .start()
        .await
        .expect("agent bastion");
    let ssh_port = ssh.get_host_port_ipv4(22.tcp()).await.unwrap();

    let config = SshTunnelConfig {
        bastion_host: "127.0.0.1".into(),
        bastion_port: ssh_port,
        auth: SshAuthMaterial::Agent(SshAgentAuth::from_socket_path("root", &sock)),
        host_key_policy: SshHostKeyPolicy::DangerousAcceptAnyForTests,
    };
    assert!(
        wait_connect(&config).await,
        "SSH agent auth to bastion must succeed"
    );

    agent_task.abort();
    let _ = std::fs::remove_file(&sock);
}

#[tokio::test]
async fn encrypted_private_key_auth_to_bastion() {
    let tag = unique_tag();
    let network = format!("tablerock-ssh-enc-{tag}");
    let bootstrap = pubkey_sshd_bootstrap(TEST_CLIENT_ENCRYPTED_PUBLIC);
    let ssh = GenericImage::new("alpine", "3.21")
        .with_exposed_port(22.tcp())
        .with_wait_for(WaitFor::message_on_stderr(
            "Server listening on 0.0.0.0 port 22",
        ))
        .with_entrypoint("sh")
        .with_cmd(["-c", bootstrap.as_str()])
        .with_network(network.as_str())
        .start()
        .await
        .expect("encrypted-key bastion");
    let ssh_port = ssh.get_host_port_ipv4(22.tcp()).await.unwrap();

    let key_auth = SshPublicKeyAuth::from_openssh_private_key_with_passphrase(
        "root",
        TEST_CLIENT_ENCRYPTED,
        Some("test-pass-phrase"),
    )
    .expect("decrypt encrypted client key");
    let config = SshTunnelConfig {
        bastion_host: "127.0.0.1".into(),
        bastion_port: ssh_port,
        auth: SshAuthMaterial::PublicKey(key_auth),
        host_key_policy: SshHostKeyPolicy::DangerousAcceptAnyForTests,
    };
    assert!(
        wait_connect(&config).await,
        "encrypted private-key auth must succeed"
    );
}

#[tokio::test]
async fn redis_driver_connects_through_local_forward_only() {
    let tag = unique_tag();
    let network = format!("tablerock-ssh-redis-{tag}");
    let redis_name = format!("tablerock-redis-{tag}");
    let _redis = GenericImage::new("redis", "8.8.0")
        .with_exposed_port(6379.tcp())
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
        .with_network(network.as_str())
        .with_container_name(redis_name.as_str())
        .start()
        .await
        .expect("redis container");
    let (_ssh, ssh_port) = start_bastion_on_network(network.as_str()).await;
    let config = accept_any_config(ssh_port);
    let tunnel = open_tunnel_retry(&config, redis_name.as_str(), 6379).await;

    let session = RedisSession::connect(
        &RedisConnectConfig::new(
            bt(LocalForwardTunnel::local_host()),
            tunnel.local_port(),
            0,
            RedisProtocol::Resp3,
            RedisTlsMode::Disable,
        ),
        RedisConnectionSecurity::new(),
    )
    .await
    .expect("RedisSession via SSH local forward");
    session
        .health_check()
        .await
        .expect("redis health through tunnel");
    let described = session
        .describe_server()
        .await
        .expect("redis describe through tunnel");
    assert!(
        described.identity().to_ascii_lowercase().contains("redis"),
        "{}",
        described.identity()
    );
    drop(tunnel);
}
