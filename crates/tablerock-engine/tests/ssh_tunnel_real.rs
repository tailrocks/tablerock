//! Real SSH bastion proof: password auth + direct-tcpip toward PostgreSQL.
//!
//! Bastion is alpine+openssh with AllowTcpForwarding (linuxserver image
//! hard-disables TCP forwarding).

use std::path::PathBuf;

use tablerock_engine::{
    SshHostKeyPolicy, SshPasswordAuth, SshTunnelConfig, SshTunnelError, channel_stream,
    connect_session, connect_session_capture_host_key, learn_host_key, open_direct_tcpip,
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

async fn start_bastion_and_pg() -> (
    testcontainers::ContainerAsync<GenericImage>,
    testcontainers::ContainerAsync<GenericImage>,
    String,
    u16,
) {
    // Unique per call so parallel tests do not collide on container names.
    let tag = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
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

    let ssh = GenericImage::new("alpine", "3.21")
        .with_exposed_port(22.tcp())
        .with_wait_for(WaitFor::message_on_stderr("Server listening on 0.0.0.0 port 22"))
        .with_entrypoint("sh")
        .with_cmd(["-c", SSHD_BOOTSTRAP])
        .with_network(network.as_str())
        .start()
        .await
        .expect("alpine sshd bastion");
    let ssh_port = ssh.get_host_port_ipv4(22.tcp()).await.unwrap();
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
