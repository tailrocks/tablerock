# Plan 012 residual — CopyRow SQL INSERT/UPDATE

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyRowSqlInsert / CopyRowSqlUpdate actions | done |
| Fail closed without base-table identity | done |
| Reuses format_copy Row scope | done |
| Unit test | done |

## Decision

Loaded-result SQL copy already existed. Row-scope SQL actions complete the
row-format matrix. UPDATE still comments WHERE-needs-PK (honest without
identity column proof on the copy path); INSERT is VALUES-complete.

## Evidence

```text
cargo test -p tablerock-tui --lib format_cursor_row
cargo test -p tablerock-tui --lib
```

## Remaining work

- WHERE clause from proven identity columns on row UPDATE (optional)
