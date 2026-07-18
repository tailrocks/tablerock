# Plan 012 residual — CopySchema / CopyBareTable

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopySchema OSC 52 schema only | done |
| CopyBareTable OSC 52 table only | done |
| Empty/missing identity no-op | done |
| Actions CopySch / CopyTblN | done |
| Unit test | done |

## Decision

CopyTbl always emits `schema.table`. Operators often need the bare
schema (SET search_path, grants) or table name alone (DDL snippets).
Separate actions avoid parsing the qualified form.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_schema_and_bare
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for schema/table part copy
