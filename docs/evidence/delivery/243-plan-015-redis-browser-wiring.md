# Plan 015 — Redis SCAN browser + key/info workbench wiring

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `Effect::ScanRedisKeys` (SCAN-only page stream) | done |
| `Effect::OpenRedisKey` via `DriverSession::redis_key_view_lines` | done |
| `Effect::LoadRedisInfo` via `redis_info_lines` | done |
| Inspector projections + namespace grouping on SCAN load | done |
| Catalog leaf activation for Redis keys | done |

## Verification

```text
cargo test -p tablerock-tui --lib
cargo test -p tablerock-cli --lib
cargo test -p tablerock-engine --lib
cargo test -p tablerock-engine --test redis_real key_type_list_stream
```
