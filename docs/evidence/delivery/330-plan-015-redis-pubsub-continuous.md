# Plan 015 residual — continuous Pub/Sub multi-page pump

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| Pump up to 8 pages / 64 lines while subscribed | done |
| `RedisSubscribePage` incremental inspector append | done |
| Grid `Streaming` during pump; `Completed` on Done | done |
| Idle gap (1.5s) after messages → honest idle stop | done |
| First-page timeout with zero messages unchanged | done |
| Stream dropped after pump (registry release) | done |
| Unit: page append + idle_stop title | done |

## Decision

Pub/Sub remains open-ended on the server. The workbench pumps a bounded
window: max 8 pages and 64 lines, with a 1.5s idle timeout between pages.
Intermediate batches paint as `RedisSubscribePage` (grid stays Streaming).
Terminal `RedisSubscribeDone` reports full lines plus `idle_stop` when the
pump stopped after messages (vs `timed_out` with zero messages). Continuous
forever-running tab is still not product scope; this closes the multi-message
gap from evidence 329.

## Evidence

```text
cargo test -p tablerock-tui --lib redis_subscribe_action
cargo check -p tablerock-cli
```

## Remaining work

- Optional: operator-controlled “listen until Cancel” without idle stop
