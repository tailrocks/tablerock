# Plan 017 residual — `russh` SSH tunnel adapter + bastion proof

Date: 2026-07-18

## Adoption

| Field | Value |
|-------|-------|
| Crate | `russh` 0.62.2 (workspace exact pin) |
| Features | `default-features = false`, `ring` |
| License | Apache-2.0 |
| Ownership | `tablerock-engine` transport adapter only; drivers stay SSH-unaware |
| Host-key policy | `DangerousAcceptAnyForTests` for Docker matrix only; production known-hosts residual |

## What landed

- `crates/tablerock-engine/src/ssh_tunnel.rs`
  - `SshPasswordAuth` Debug redacts password
  - `connect_session` password auth via russh client handle
  - `open_direct_tcpip` / `channel_stream` for driver-facing streams
  - `spawn_local_forward` binds `127.0.0.1:0` and bridges one accept over direct-tcpip
- Real Docker test `ssh_tunnel_real`:
  - alpine+openssh bastion (`AllowTcpForwarding yes`)
  - shared network DNS to postgres
  - SSLRequest through direct-tcpip and through local forward → `N`/`S`

Fixture note: `lscr.io/linuxserver/openssh-server` ships `AllowTcpForwarding no`,
which rejects direct-tcpip; alpine bootstrap is the intentional test bastion.

## Commands

```bash
cargo test -p tablerock-engine ssh_tunnel
cargo test -p tablerock-engine --test ssh_tunnel_real
```

## Residual

- Known-hosts fail-closed → evidence 261
- Agent/key auth modes
- Profile aggregate SSH section + TUI editor fields
- Wire tunnels under connect path for PG/CH/Redis drivers
- Full multi-engine bastion matrix
