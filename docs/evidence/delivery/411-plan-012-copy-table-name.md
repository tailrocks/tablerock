# Plan 012 residual — CopyTableName

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| Action CopyTbl | done |
| Copies `schema.table` when base identity known | done |
| No-op without base table | done |
| Unit test | done |

## Decision

SQL authoring often needs the qualified relation. Fail closed when browse
did not prove base_schema/base_table.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_table_name
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for table name copy
