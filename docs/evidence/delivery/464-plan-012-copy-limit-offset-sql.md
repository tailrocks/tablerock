# Plan 012 residual — CopyLimitOffsetSql / CopySelectPageSql

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `limit_offset_sql` from row_count/start_row | done |
| CopyLimitOffsetSql fragment | done |
| CopySelectPageSql SELECT+WHERE?+ORDER?+LIMIT | done |
| Empty page no-op | done |
| Actions CopyLim / CopySelP | done |
| Unit tests | done |

## Decision

Resident window identity is `start_row` + `row_count`. Operators often
want to re-issue the same page bounds as SQL. CopySelP composes the full
visible projection scaffold with optional filters/sort and LIMIT/OFFSET.

## Evidence

```text
cargo test -p tablerock-tui --lib limit_offset_sql
cargo test -p tablerock-tui --lib copy_limit_offset_and_select_page
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for LIMIT/OFFSET page scaffold
