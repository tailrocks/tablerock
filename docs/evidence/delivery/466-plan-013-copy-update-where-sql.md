# Plan 013 residual — CopyUpdateWhereSql

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| UPDATE SET cursor-col = typed literal | done |
| WHERE from identity locator | done |
| Fail closed without identity WHERE | done |
| Fail closed when cursor is identity col | done |
| Action CopyUpdW | done |
| Unit test | done |

## Decision

Presentation-only. Complements CopyRowSqlUpdate (full row) with a single
cell SET scaffold. Never updates the identity column itself (no-op).

## Evidence

```text
cargo test -p tablerock-tui --lib copy_update_where_sql
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for UPDATE WHERE scaffold
