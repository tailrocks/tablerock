# Plan 017 residual — profile SSH property bindings

Date: 2026-07-18

## What landed

Optional SSH tunnel fields on `ProfileProperty` (schema ordinals 11–16):

| Property | Kind | Notes |
|----------|------|--------|
| `SshHost` | literal | Presence marks tunnel requested |
| `SshPort` | literal port 1..=65535 | |
| `SshUsername` | literal | |
| `SshPassword` | secret only | |
| `SshPrivateKey` | secret only | OpenSSH private key material |
| `SshKnownHostsPath` | literal path | Fail-closed known_hosts file |

Persistence encode/decode extended. Stale persistence `schema_version` test
expectations updated 7→10 (migrations 0008–0010 already on trunk).

## Commands

```bash
cargo test -p tablerock-core --test profile
cargo test -p tablerock-persistence --tests
```

## Residual

- Resolve SSH secrets → `SshTunnelConfig` on connect path
- TUI connection editor SSH section
- Agent auth / encrypted key passphrase
