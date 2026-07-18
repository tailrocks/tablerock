# Plan 015 residual — isolated BLPOP from command editor

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| Engine `blocking_pop` disposable connection (pre-existing) | done |
| Lone `BLPOP`/`BRPOP` + key → `Effect::RedisBlockingPop` | done |
| Mixed pipeline still denies blocking on shared path | done |
| CLI pumps `DriverPageRequest::RedisBlockingPop` | done |
| Unit: emit isolated effect; mixed deny | done |

## Decision

Shared-session multiplex must never run blocking commands. A **single**
operator BLPOP/BRPOP with a key is routed to the existing disposable-
connection `blocking_pop` stream (CLIENT ID + cancel registry). Mixed
scripts with blocking remain denied. Timeout is server-side BLPOP 0;
cancel uses the session cancel path. First key only for multi-key forms.

## Evidence

```text
cargo test -p tablerock-tui --lib redis_run_pipeline
cargo check -p tablerock-cli
```

## Remaining work

- Pub/Sub UI (post-parity)
- Multi-key BRPOP / finite timeout arg surface
