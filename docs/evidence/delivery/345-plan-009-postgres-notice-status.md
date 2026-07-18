# Plan 009 residual — PostgreSQL NOTICE → grid status

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `PostgresSession::try_drain_notices` non-blocking drain | done |
| `DriverSession::drain_server_notices` (PG impl; others empty) | done |
| After `ExecuteSql` stream: drain notices into `GridStreamComplete` | done |
| Empty result path: notice text on `server_progress` | done |
| TUI shows `notice: …` on grid status (`error_label`) | done |
| Redacted lines: severity + message only | done |

## Decision

Notices are best-effort post-stream drain (max 8). Overflow surfaces as a
single line. No inspector panel yet; status bar is enough for Phase 4/9
notice visibility without a second model.

## Evidence

```text
cargo test -p tablerock-tui --lib grid_stream_complete
cargo check -p tablerock-engine -p tablerock-cli -p tablerock-tui
```

## Remaining work

- Inspector notice history queue (optional)
- Association of notices to multi-statement ordinals
