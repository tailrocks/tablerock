# Plan 012 residual — CopyCountSql

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| SELECT count(*) FROM schema.table | done |
| Needs base-table identity | done |
| Action CopyCnt | done |
| Unit test | done |

## Decision

Exact totals sometimes need a re-query. CopyCnt scaffolds
`SELECT count(*) FROM "schema"."table"` without filters (operators
append WHERE from chips if needed).

## Evidence

```text
cargo test -p tablerock-tui --lib copy_count_sql
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for count scaffold copy
