# Plan 006 residual — session health probe + auto reconnect

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `CheckSessionHealth` → real `session.health()` | done |
| `HealthOk` / `HealthFailed` update session status | done |
| `HealthFailed` + BoundedAutomatic → ReconnectSession attempt 0 | done |
| Manual preference does not auto-reconnect | done |
| Workbench Health + Reconn actions | done |
| `last_connect_draft` retained on Connect/Reconnect | done |
| Unit: auto vs manual | done |

## Decision

Health is operator-triggered (Health action) rather than a timer in the
reducer. Failed health with `reconnect_preference` matching
`should_auto_reconnect` starts bounded reconnect from `last_connect_draft`.
Manual preference only updates status; operator uses Reconn.

## Evidence

```text
cargo test -p tablerock-tui --lib health_failed_auto
cargo test -p tablerock-tui --lib reconnecting_message
cargo check -p tablerock-cli
```

## Remaining work

- ~~Load reconnect preference from saved profile aggregate on ConnectProfile~~
  (closed: evidence 334)
