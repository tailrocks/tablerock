# Plan 017 residual — SSH known_hosts fail-closed

Date: 2026-07-18

## What landed

- `SshHostKeyPolicy::KnownHostsPath(PathBuf)` — OpenSSH known_hosts via
  `russh::keys::check_known_hosts_path` (host+port from bastion config)
- Empty / missing entry → `SshTunnelError::HostKeyRejected` (maps
  `russh::Error::UnknownKey`)
- Key-changed for same host:port → reject (fail closed)
- `learn_host_key` + `connect_session_capture_host_key` for bootstrap/tests
- Unit tests: empty reject, learn/check round-trip, OpenSSH line format
- Real Docker: empty reject, learn then accept, wrong key reject

## Commands

```bash
cargo test -p tablerock-engine ssh_tunnel
cargo test -p tablerock-engine --test ssh_tunnel_real
```

## Residual

- Agent/key auth modes
- Profile aggregate SSH section + connect-path wiring under drivers
- Multi-engine bastion matrix
