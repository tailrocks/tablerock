# Plan 015 — Redis type views, streams, command classification

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `key_type` TYPE → RedisKeyKind | done |
| `list_range` LRANGE bounded | done |
| `stream_range` XRANGE bounded | done |
| `execute_command_argv` + blocking deny | done |
| TUI `redis_key_view` six-kind projections | done |
| TUI `redis_command` tokenize/classify/complete | done |
| Docker: type/list/stream/info/PING/BLPOP deny | done |
| Full workbench key-browser wiring | residual |
| Full multi-type staged edits beyond SET/DEL/TTL | residual |

## Verification

```text
cargo test -p tablerock-tui --lib redis_
cargo test -p tablerock-engine --lib scan_policy
cargo test -p tablerock-engine --test redis_real key_type_list_stream
```

## Provenance

Redis TYPE/LRANGE/XRANGE/INFO command docs; plan 015 product redis.md.
Command name table is a curated subset for safety (not a vendored redis-doc dump).
