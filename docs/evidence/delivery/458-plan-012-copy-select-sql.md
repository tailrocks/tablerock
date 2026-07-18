# Plan 012 residual — CopySelectSql

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| SELECT visible cols FROM schema.table | done |
| All identifiers SQL-quoted | done |
| Needs base-table identity | done |
| Layout-aware (Solo/hide) | done |
| Action CopySel | done |
| Unit test | done |

## Decision

Operators often scaffold a re-query from the current grid projection.
CopySel emits a two-line SELECT using visible columns and base identity
facts (presentation aid only — not executed).

## Evidence

```text
cargo test -p tablerock-tui --lib copy_select_sql
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for SELECT scaffold copy
