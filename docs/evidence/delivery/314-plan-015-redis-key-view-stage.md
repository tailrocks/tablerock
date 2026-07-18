# Plan 015 residual — stage Redis collection mutations from key view

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `redis_stage_target` + `redis_staged` on workbench | done |
| OpenRedisKey sets stage target (logical DB + key + kind) | done |
| `StageRedisAdd` / `StageRedisRemove` confirm dialogs | done |
| Parse hset/hdel/sadd/srem/zadd/zrem paste buffers | done |
| `ReviewMutations` collects redis_staged specs | done |
| DiscardStaged + MutationApplied clear redis staged | done |
| Unit: hash stage → review effect; parse helpers | done |

## Decision

Collection applies already had typed plans (312) and presentation specs (313).
Operators now stage from an open Redis hash/set/zset key view: paste a
payload, review, apply via the same handle path. Relational grid drafts are
unchanged and unused on Redis.

## Evidence

```text
cargo test -p tablerock-tui --lib redis_stage
cargo test -p tablerock-tui --lib stage_redis_hash
cargo test -p tablerock-cli --lib redis_collection_spec
```

## Remaining work

- Collection next-page affordance beyond first-page preview
- Full command editor tab + pipeline outcomes UI
