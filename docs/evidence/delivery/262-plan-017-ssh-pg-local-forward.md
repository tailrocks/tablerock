# Plan 017 residual — PostgreSQL driver through SSH local forward

Date: 2026-07-18

## What landed

- `LocalForwardTunnel` + `open_local_forward_tunnel` — multi-accept loop;
  drivers only see `127.0.0.1:local_port`
- `spawn_local_forward` accepts concurrent bridges (per-accept task)
- Real Docker proof: `PostgresSession::connect` + health + describe_server
  through bastion direct-tcpip local forward (no shell; driver SSH-unaware)

## Commands

```bash
cargo test -p tablerock-engine --test ssh_tunnel_real
```

## Residual

- Profile aggregate SSH properties + TUI section
- Connect-path auto-wrap from resolved profile SSH settings
- ClickHouse/Redis through same tunnel helper
- Agent/key auth
