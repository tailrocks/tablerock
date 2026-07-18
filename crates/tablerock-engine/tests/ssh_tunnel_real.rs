//! Real SSH bastion proof: password auth + direct-tcpip toward PostgreSQL.
//!
//! Bastion is alpine+openssh with AllowTcpForwarding (linuxserver image
//! hard-disables TCP forwarding).

use tablerock_engine::{
    SshHostKeyPolicy, SshPasswordAuth, SshTunnelConfig, channel_stream, connect_session,
    open_direct_tcpip, spawn_local_forward,
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

#[tokio::test]
async fn password_auth_and_direct_tcpip_to_postgres() {
    let network = format!("tablerock-ssh-{}", std::process::id());
    let pg_name = format!("tablerock-pg-{}", std::process::id());

    let _postgres = GenericImage::new("postgres", "18.4-alpine")
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

    // direct-tcpip from bastion to postgres container (Docker DNS).
    let channel = open_direct_tcpip(&handle, &pg_name, 5432)
        .await
        .expect("direct-tcpip channel to postgres on shared network");
    let mut stream = channel_stream(channel);

    // SSLRequest: postgres answers 'N' (refuse) or 'S' (accept) — proves tunnel.
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

    // Local-forward listener form (drivers bind 127.0.0.1:port).
    let handle2 = connect_session(&config).await.expect("second auth");
    let (local_port, join) = spawn_local_forward(handle2, pg_name, 5432)
        .await
        .expect("local forward bind");
    assert!(local_port > 0);

    // Prove local forward carries the same SSLRequest/response.
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
