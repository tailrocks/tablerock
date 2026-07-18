# Plan 012 residual — CopyFilterWhereSql / CopySelectFilterSql

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `filters_where_sql` from chips + raw WHERE | done |
| Page-local quick filter excluded | done |
| Known operators only (fail closed) | done |
| Quote idents + escape string literals | done |
| CopyFilterWhereSql fragment | done |
| CopySelectFilterSql SELECT+WHERE[+ORDER] | done |
| Unit tests | done |

## Decision

CopyFilt is chip-bar text. SQL re-query needs a WHERE from typed chips.
Presentation-only literals (not plan parameters). Raw WHERE is
parenthesized AND-composed. Unknown operators are skipped fail-closed.

## Evidence

```text
cargo test -p tablerock-tui --lib filters_where_sql
cargo test -p tablerock-tui --lib copy_filter_where_and_select_filter
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for filter WHERE SQL scaffold
