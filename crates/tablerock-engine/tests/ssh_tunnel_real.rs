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
    SshHostKeyPolicy, SshPasswordAuth, SshTunnelConfig, SshTunnelError, channel_stream,
    connect_session, connect_session_capture_host_key, learn_host_key, open_direct_tcpip,
    open_local_forward_tunnel, spawn_local_forward,
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
        .with_wait_for(WaitFor::message_on_stderr("Server listening on 0.0.0.0 port 22"))
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
        auth: SshPasswordAuth::new("root", "tunnel-pass"),
        host_key_policy: SshHostKeyPolicy::DangerousAcceptAnyForTests,
    }
}

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

    let config = SshTunnelConfig {
        bastion_host: "127.0.0.1".into(),
        bastion_port: ssh_port,
        auth: SshPasswordAuth::new("root", "tunnel-pass"),
        host_key_policy: SshHostKeyPolicy::DangerousAcceptAnyForTests,
    };

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
        auth: SshPasswordAuth::new("root", "tunnel-pass"),
        host_key_policy: SshHostKeyPolicy::KnownHostsPath(known_hosts.clone()),
    };

    // Wait for bastion readiness via accept-any first.
    let mut live_key = None;
    for _ in 0..40 {
        let accept_any = SshTunnelConfig {
            bastion_host: "127.0.0.1".into(),
            bastion_port: ssh_port,
            auth: SshPasswordAuth::new("root", "tunnel-pass"),
            host_key_policy: SshHostKeyPolicy::DangerousAcceptAnyForTests,
        };
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
        auth: SshPasswordAuth::new("root", "tunnel-pass"),
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
        auth: SshPasswordAuth::new("root", "tunnel-pass"),
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
    session.health_check().await.expect("redis health through tunnel");
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
