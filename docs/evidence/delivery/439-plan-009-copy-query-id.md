# Plan 009 residual — CopyQueryId

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyQueryId OSC 52 when set | done |
| Missing/empty no-op | done |
| Action CopyQid | done |
| Unit test | done |

## Decision

ClickHouse (and similar) surfaces `server_query_id` on the grid status line.
Operators need the bare id for `KILL QUERY` / logs without parsing status.
CopyQid copies only that field.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_query_id
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for query-id copy
