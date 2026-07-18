# Plan 015 residual — multi-type Redis collection mutations

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `MutationChange` HSET/HDEL/SADD/SREM/ZADD/ZREM | done |
| Validation: empty field/member fail-closed; Redis-only engine gate | done |
| Sequential apply (non-transactional, no rollback language) | done |
| Debug omits payload bytes (length only) | done |
| Core unit + Docker real apply matrix | done |
| TUI stage/review UI for collection edits | residual |

## Decision

String SET/DEL/TTL already used the authorized mutation seam. Collection
mutations join the same sequential Redis apply path with explicit command
markers in outcomes (`command=HSET` …). ZADD scores travel as IEEE bits;
non-finite scores fail closed without a network call.

## Evidence

```text
cargo test -p tablerock-core --test mutation redis
cargo test -p tablerock-engine --test redis_real applies_multi_type_collection
```

## Remaining work

- TUI MutationChangeSpec + stage actions for hash/set/zset cells
- Full command editor tab + pipeline outcomes UI
- Collection next-page affordance beyond first-page preview
