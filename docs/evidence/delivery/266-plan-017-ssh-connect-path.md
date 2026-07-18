# Plan 017 residual — SSH tunnel on connect path

Date: 2026-07-18

## What landed

- `ConnectionDraft` SSH fields (host/port/user/password/key/known_hosts)
- `aggregate_to_draft` / `draft_to_aggregate` round-trip SSH property bindings
- `open_described_session` opens `LocalForwardTunnel` when `ssh_host` set:
  - requires known_hosts path (fail closed)
  - password or public-key auth
  - rewrites driver endpoint to `127.0.0.1:local_port`
- `SessionRegistry::register_with_tunnel` keeps tunnel alive until disconnect
- Test/reconnect paths pass tunnel through; no shell

## Commands

```bash
cargo test -p tablerock-engine --lib
cargo test -p tablerock-tui --lib
cargo test -p tablerock-cli --lib
```

## Residual

- TUI connection editor SSH section
- Agent auth / encrypted key passphrase
- End-to-end Docker test through CLI connect effect (draft with SSH fields)
