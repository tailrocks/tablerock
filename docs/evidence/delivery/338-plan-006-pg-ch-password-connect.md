# Plan 006 residual — PostgreSQL and ClickHouse password on connect

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `PostgresSession::connect_with_password` | done |
| `PostgresSession::connect_with_tls_password` | done |
| Empty/`None` password omits driver password (trust/peer) | done |
| `ClickHouseSession::connect_with_password` → `Client::with_password` | done |
| TUI/CLI `open_driver_session` passes resolved password to PG/CH | done |
| UniFFI bridge connect passes password for all three engines | done |
| Existing no-password call sites keep working via wrappers | done |

## Decision

Password material is never stored on connect config (Debug stays redacted).
Optional password is a call-time argument only. Redis already used
`RedisCredentials`; PG/CH now match that attempt-scoped pattern. Resolution
sources (prompt, plaintext, env, 1Password) feed the same `resolved_password`
path used for Redis.

## Evidence

```text
cargo check -p tablerock-engine -p tablerock-cli -p tablerock-ffi
```

Docker real-server suites continue to use trust auth without passwords.

## Remaining work

- Keychain native source
- 1Password metadata picker
- Optional: zeroize `resolved_password` String after connect returns
