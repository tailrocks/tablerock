# Real-server describe_server matrix (Test Connection facts)

Date: 2026-07-18

## Checkpoint

Plan 006. `describe_server` returns bounded identity/version facts on live
PostgreSQL 18.4, ClickHouse 26.3, and Redis 8.8 fixtures. This is the engine
side of Test Connection without save.

## Decision

- Integration test crate file `tests/describe_server_real.rs` (Docker via
  testcontainers).
- PG: `SELECT version()` trust auth.
- CH: `SELECT version()` HTTP; retry until ready (lazy connect).
- Redis: `INFO server` → `redis_version`.

## Evidence

```
cargo test -p tablerock-engine --test describe_server_real
# 3 passed (postgres, clickhouse, redis) in ~2s wall with warm images
```

Log: implementer scratch `describe_server_real.log`.

## Remaining work

- End-to-end CLI effect TestConnection against same fixtures.
- Group CRUD dialogs; removal safety; Phase 3 ROADMAP exit.
