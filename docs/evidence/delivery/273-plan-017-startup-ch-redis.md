# Plan 017 residual — ClickHouse and Redis startup executors

Date: 2026-07-18

## What landed

- `run_clickhouse_startup_actions` — ReadOnly SQL under timeout; Write skipped
- `run_redis_startup_actions` — whitespace-tokenized command argv; Dangerous skipped
- Connect path wires all three engines after open
- Docker proofs for CH SELECT/skip and Redis PING/skip FLUSHDB

## Commands

```bash
cargo test -p tablerock-engine --test startup_actions_real
cargo test -p tablerock-cli --lib
```

## Residual

- TUI startup action editor
- Surface StartupRunReport in connect UI
