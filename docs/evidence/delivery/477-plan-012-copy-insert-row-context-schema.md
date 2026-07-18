# Plan 012 residual — CopyInsertRowSql / CopyContextSchema

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyInsertRowSql full INSERT+VALUES | done |
| Identity-gated fail closed | done |
| CopyContextSchema from context bar | done |
| Distinct from base-table CopySchema | done |
| Actions CopyInsR / CopyCtxSch | done |
| Unit tests | done |

## Decision

CopyIns + CopyVals are composable; operators also want one-shot
`INSERT … VALUES (row)`. Context-bar schema is the session schema switcher
value, not necessarily the active browse table's base_schema.

## Evidence

```text
cargo test -p tablerock-tui --lib format_insert_and_values
cargo test -p tablerock-tui --lib copy_insert_and_values
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for insert-row / context schema copy
