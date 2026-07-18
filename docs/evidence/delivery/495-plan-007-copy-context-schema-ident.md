# Plan 007 residual — CopyContextSchemaIdent

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyContextSchemaIdent SQL-quoted | done |
| Fail closed when no context schema | done |
| Distinct from base-table CopySchQ | done |
| Action CopyCtxSchQ | done |
| Unit test | done |

## Decision

Context-bar schema (session switcher) may differ from browse base_schema.
Quoted form mirrors CopySchQ for SET search_path / qualified fragments.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_insert_and_values
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for context schema ident copy
