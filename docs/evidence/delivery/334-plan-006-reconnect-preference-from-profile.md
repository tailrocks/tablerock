# Plan 006 residual — reconnect preference from profile on connect

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `ConnectionDraft.reconnect_preference` | done |
| Loaded from `ProfilePreferences::reconnect` on profile draft | done |
| `ConnectOk.reconnect_preference` sets model field | done |
| HealthFailed auto-reconnect uses that field | done (333) |
| Unit: ConnectOk BoundedAutomatic sets preference | done |

## Decision

Saved profiles carry reconnect preference in Turso. On ConnectProfile the
aggregate maps Manual / BoundedAutomatic into the draft and ConnectOk, so
Health auto-reconnect matches the saved policy without a second preference
lookup.

## Evidence

```text
cargo test -p tablerock-tui --lib health_failed_auto
cargo check -p tablerock-cli
```

## Remaining work

- None for this residual
