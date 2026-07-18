# Plan 015 residual — Redis collection MutationChangeSpec + review target

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `MutationChangeSpec` HSET/HDEL/SADD/SREM/ZADD/ZREM | done |
| CLI `typed_changes_from_specs` rebuilds typed changes | done |
| `review_mutations` picks PG/CH/Redis target from session engine | done |
| Redis target: logical DB from database field, key from table field | done |
| Preview lines label HSET/… without payloads | done |
| Unit: map specs + reject empty/nonfinite | done |

## Decision

Presentation stays free of `BoundedBytes`/`MutationChange` types: TUI emits
plain string specs; CLI binds bytes and builds Redis targets when the live
session is Redis. ClickHouse review also targets `ClickHouseTable` (was
PG-only hardcode).

## Evidence

```text
cargo test -p tablerock-cli --lib redis_collection_spec
cargo test -p tablerock-core --test mutation redis
```

## Remaining work

- Workbench actions to stage collection specs from OpenRedisKey lines
