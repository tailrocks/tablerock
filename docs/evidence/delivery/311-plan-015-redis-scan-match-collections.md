# Plan 015 residual — SCAN MATCH + collection first-page views

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `RedisKeyScan.match_pattern` + engine `SCAN … MATCH` | done |
| CLI applies MATCH (empty/`*` → no MATCH clause) | done |
| TUI ScanKeys uses catalog filter as MATCH pattern | done |
| OpenRedisKey: Hash/Set/ZSet first page via HSCAN/SSCAN/ZSCAN | done |
| Docker: MATCH `scan-*` filters keys; HSCAN/SSCAN lines | done (redis_real) |
| Unit: catalog filter → ScanRedisKeys pattern | done |

## Decision

Key browse stays SCAN-only (never KEYS). Optional MATCH is bound bytes on
the SCAN command. Collection key views call the existing `scan_collection`
stream and project the first bounded page into inspector lines so hash/set/
zset keys are inspectable without HGETALL/SMEMBERS.

## Evidence

```text
cargo test -p tablerock-tui --lib scan_redis_keys_uses_catalog
cargo test -p tablerock-engine --test redis_real
```

## Remaining work

- Full command editor tab + pipeline outcomes UI
- Multi-type staged hash/list/set/zset edits beyond SET/DEL/TTL
- Collection next-page affordance beyond first-page preview
