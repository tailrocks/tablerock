# Plan 012 residual — CopyColumn resident values

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `format_cursor_column` one value per resident row | done |
| NULL → `NULL` token (same as result copy) | done |
| Action CopyCol | done |
| Unit test | done |

## Decision

Vertical extract of the cursor column for scripting/paste. Resident page
only (no server re-query); pending cells skipped.

## Evidence

```text
cargo test -p tablerock-tui --lib format_cursor_column
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for column value copy
