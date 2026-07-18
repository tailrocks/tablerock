# Plan 017 residual — SSH local-forward three-engine matrix

Date: 2026-07-18

## What landed

Docker bastion (alpine+openssh, TCP forwarding on) + shared Docker network
proves each driver through `LocalForwardTunnel` only:

| Engine | Target port | Proof |
|--------|-------------|--------|
| PostgreSQL | 5432 | health + describe_server (evidence 262) |
| ClickHouse | 8123 | describe_server retry |
| Redis | 6379 | health + describe_server |

Drivers receive only `127.0.0.1:local_port`. No shell, no bastion secrets on
driver configs.

## Commands

```bash
cargo test -p tablerock-engine --test ssh_tunnel_real
```

## Residual

- Profile aggregate SSH properties + TUI section + connect-path auto-wrap
- Agent/key auth modes
