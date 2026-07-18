# Plan 013 residual — CopySelectWhereSql

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| SELECT visible + FROM + WHERE locator | done |
| Needs base table + identity locator | done |
| Action CopySelW | done |
| Unit test | done |

## Decision

CopySel is whole-table SELECT. Point-query paste needs the cursor row
locator WHERE from identity facts. CopySelW appends `cursor_where_sql()`.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_select_where_sql
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for SELECT…WHERE scaffold
