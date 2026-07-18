# Plan 012 residual — CopyOrderBySql / CopySelectOrderSql

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `order_by_sql` multi-key ASC/DESC | done |
| CopyOrderBySql fragment only | done |
| CopySelectOrderSql SELECT+FROM+ORDER | done |
| Empty sort no-op | done |
| Actions CopyOrd / CopySelO | done |
| Unit tests | done |

## Decision

CopySort is chip-bar text. SQL re-query needs a real ORDER BY clause.
`order_by_sql` quotes identifiers and maps Asc/Desc; CopySelO composes
it with the visible-column SELECT scaffold.

## Evidence

```text
cargo test -p tablerock-tui --lib order_by_sql
cargo test -p tablerock-tui --lib copy_order_by_and_select_order
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for ORDER BY scaffold copy
