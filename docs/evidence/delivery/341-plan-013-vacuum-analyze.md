# Plan 013 residual — VACUUM / ANALYZE maintenance gates

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `ActionId::VacuumTable` / `AnalyzeTable` + toolbar actions | done |
| Exact table-name re-type confirm (fail closed) | done |
| `ExecuteTableOp` ops `vacuum` / `analyze` | done |
| CLI fixed SQL: `VACUUM schema.table` / `ANALYZE schema.table` via `quote_ident` | done |
| Unit tests for wrong-name reject + correct dispatch | done |

## Decision

Maintenance ops use the same exact-name gate as truncate/drop (not free SQL).
No `VACUUM FULL` or options in v1 — table-scoped only. Engine statement
stream does not wrap BEGIN, so VACUUM is valid outside a transaction.

## Evidence

```text
cargo test -p tablerock-tui --lib vacuum_table
cargo test -p tablerock-tui --lib analyze_table
cargo check -p tablerock-cli
```

## Remaining work

- Optional: VACUUM (ANALYZE), VERBOSE status projection
- Copied DDL export from structure inspector
