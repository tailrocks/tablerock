# Plan 012 residual — CopyInsertLoadedSql

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyInsertLoadedSql multi-row VALUES | done |
| Identity-gated | done |
| Cap 500 resident rows | done |
| Pending cells fail closed | done |
| Action CopyInsL | done |
| Unit tests | done |

## Decision

Paste multi-row INSERT for the resident window without streaming export.
Hard cap 500 rows keeps OSC 52 payloads bounded; full export stays on the
Export path.

## Evidence

```text
cargo test -p tablerock-tui --lib format_insert_and_values
cargo test -p tablerock-tui --lib copy_insert_and_values
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for loaded INSERT scaffold
