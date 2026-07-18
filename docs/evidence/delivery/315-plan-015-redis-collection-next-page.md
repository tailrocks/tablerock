# Plan 015 residual — Redis collection next-page (RMore)

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `redis_key_view_lines(key, collection_skip)` | done |
| Skip/take rescan (no held stream state across effects) | done |
| `next_collection_skip` on RedisKeyViewLoaded | done |
| TUI `RedisCollectionMore` / RMore action | done |
| Docker: large set first page has next; second page differs | done |
| Unit: RMore emits OpenRedisKey with skip | done |

## Decision

Collection streams are not retained across TEA effect turns. Next-page
rescans from the start and skips already-seen entries (bounded take=32).
Honest and fail-closed; not O(1) cursor resume, but correct without KEYS.

## Evidence

```text
cargo test -p tablerock-tui --lib redis_collection_more
cargo test -p tablerock-engine --test redis_real collection_page_skip
```

## Remaining work

- Full command editor tab + pipeline outcomes UI
