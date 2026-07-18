# Plan 012 residual — CopyCellSql

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `format_cursor_cell_sql` | done |
| NULL → `NULL` | done |
| Number unquoted when parseable | done |
| Boolean TRUE/FALSE | done |
| Text quoted; `'` escaped | done |
| Action CopySql | done |
| Unit test | done |

## Decision

CopyCell is raw presentation; CopyHex is binary hex. SQL snippet paste
needs typed literals. CopySql reuses the same quote rules as INSERT/UPDATE
formatters for text, with number/boolean/null distinctions.

## Evidence

```text
cargo test -p tablerock-tui --lib format_cursor_cell_sql
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for cell SQL literal copy
