# Plan 013 residual — CopyPkIdents

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyPkIdents comma-separated quoted | done |
| Empty identity no-op | done |
| Escapes embedded `"` | done |
| Action CopyPkQ | done |
| Unit test | done |

## Decision

CopyPk is raw TSV names. SQL PRIMARY KEY / join lists need
`"col1", "col2"` form. CopyPkQ reuses `quote_ident_sql`.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_pk_idents
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for pk SQL ident copy
