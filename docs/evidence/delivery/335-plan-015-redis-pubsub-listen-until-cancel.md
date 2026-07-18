# Plan 015 residual — Pub/Sub listen until Cancel

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| First page still 2s timeout (empty channel honest) | done |
| After first message: block on next page (no idle stop) | done |
| Cancel maps to Done `cancelled` (not Failed) | done |
| Max 64 pages / 256 lines hard bound | done |
| Inspector title `· cancelled` | done |
| Unit: cancelled Done paints cancel state | done |

## Decision

Idle-stop after first message was too aggressive for live Pub/Sub. Once any
message arrives the pump waits on the isolated subscription stream until
operator Cancel (session cancel → subscription cancel_requested), stream
end, or hard line/page caps. Empty-channel first wait remains 2s so Sub
without traffic still completes.

## Evidence

```text
cargo test -p tablerock-tui --lib redis_subscribe_action
cargo check -p tablerock-cli
```

## Remaining work

- None for listen-until-Cancel residual
