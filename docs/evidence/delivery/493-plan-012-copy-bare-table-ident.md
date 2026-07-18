# Plan 012 residual — CopyBareTableIdent

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyBareTableIdent SQL-quoted bare table | done |
| Fail closed without base_table | done |
| Action CopyTblNQ | done |
| Unit test | done |

## Decision

CopyTblN is bare name; quoted-only table (no schema) is needed for engines
without schemas or for RENAME/ALTER fragments. Completes SchQ/TblNQ pair.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_schema_and_bare_table
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for bare table ident copy
