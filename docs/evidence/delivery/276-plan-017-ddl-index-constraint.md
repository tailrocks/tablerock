# Plan 017 residual — DDL create/drop index and constraint

Date: 2026-07-18

## What landed

- `DdlPlan` validation requires object name for index/constraint kinds;
  CreateIndex/AddConstraint require `type_text` (column list / clause body)
- `PostgresSession::execute_ddl_plan` implements:
  - `CREATE INDEX name ON schema.table (cols)` — simple ident columns only
  - `DROP INDEX schema.name`
  - `ADD CONSTRAINT` for `UNIQUE` / `PRIMARY KEY` / `CHECK` bodies (charset-restricted)
  - `DROP CONSTRAINT`
- Docker: create index, add UNIQUE, reject duplicate, drop constraint, drop index

## Commands

```bash
cargo test -p tablerock-core ddl
cargo test -p tablerock-engine --test postgres_real ddl_index
```

## Residual

- TUI DDL review/authorize UI for typed plans
- Reindex kind execution path
