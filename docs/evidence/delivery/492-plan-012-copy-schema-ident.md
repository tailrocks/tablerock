# Plan 012 residual — CopySchemaIdent

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopySchemaIdent SQL-quoted base schema | done |
| Fail closed without base_schema | done |
| Action CopySchQ | done |
| Unit test | done |

## Decision

CopySch is bare schema text; SQL scaffolds often need the quoted form alone
(e.g. SET search_path). Mirrors CopyDbQ / CopyColQ pattern.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_schema_and_bare_table
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for schema ident copy
