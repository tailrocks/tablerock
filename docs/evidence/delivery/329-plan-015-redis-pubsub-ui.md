# Plan 015 residual — Redis Pub/Sub UI (subscribe / psubscribe)

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| Actions Sub / PSub + confirm channel/pattern | done |
| Effect `RedisSubscribe` isolated connection | done |
| CLI: `DriverPageRequest::RedisSubscribe` + 2s first-page wait | done |
| Inspector paints channel · payload lines | done |
| Timeout with zero messages is honest (not failure) | done |
| Unit: confirm → effect → done paints inspector | done |

## Decision

Pub/Sub uses the engine’s isolated subscription stream (not the shared
multiplex). The TUI collects the first bounded page (or 2s timeout) then
drops the stream — continuous fan-out UI remains future polish. Cancel
still goes through the session cancel path while the stream is live.

## Evidence

```text
cargo test -p tablerock-tui --lib redis_subscribe_action
cargo check -p tablerock-cli
```

## Remaining work

- ~~Continuous streaming tab / multi-message pump after first page~~
  (closed: evidence 330)
