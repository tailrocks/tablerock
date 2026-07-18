# Plan 006 residual — reconnect auto re-dispatch with delayed sleep

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `Reconnecting` carries `draft` for next attempt | done |
| Update re-dispatches `Effect::ReconnectSession` | done |
| CLI sleeps `next_backoff_ms(attempt)` when attempt > 0 | done |
| Auth failures still stop without further attempts | done |
| Action `Reconnect` starts attempt 0 from editor draft | done |
| Unit: Reconnecting → ReconnectSession re-dispatch | done |

## Decision

Backoff delays belong in the executor (TEA: no sleep in update). On a
non-auth failure the executor returns `Reconnecting` with the next attempt
index, delay fact, and draft; the reducer immediately re-dispatches
`ReconnectSession`. Attempt N>0 sleeps `next_backoff_ms(N)` before connect.
Attempt 0 is immediate. Budget remains attempts 0..=5.

## Evidence

```text
cargo test -p tablerock-tui --lib reconnecting_message
cargo test -p tablerock-tui --lib reconnect::
cargo check -p tablerock-cli
```

## Remaining work

- ~~Auto-start reconnect from health failure when preference is BoundedAutomatic~~
  (closed: evidence 333)
