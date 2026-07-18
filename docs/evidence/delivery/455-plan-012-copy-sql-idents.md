# Plan 012 residual — CopyColumnIdent / CopyTableIdent

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyColumnIdent via `quote_ident_sql` | done |
| CopyTableIdent `schema.table` quoted | done |
| Escapes embedded `"` | done |
| Actions CopyColQ / CopyTblQ | done |
| Unit test | done |

## Decision

CopyColN/CopyTbl emit raw names. SQL paste needs double-quoted identifiers
with `"` doubled. Reuses structure DDL `quote_ident_sql` (no second quoter).

## Evidence

```text
cargo test -p tablerock-tui --lib copy_column_and_table_idents
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for SQL identifier copy
