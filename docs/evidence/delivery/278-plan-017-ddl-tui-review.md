# Plan 017 residual — DDL TUI review and execute path

Date: 2026-07-18

## What landed

- `ActionId::DdlAddColumn` / `DdlCreateIndex` open `ConfirmDialog::DdlReview`
  with typed-plan preview (no free SQL paste as the plan body)
- Confirm buffer formats:
  - add_column: `"col_name type"`
  - create_index: `"index_name column"`
- Incomplete buffer fail-closed (no effect)
- `Effect::ExecuteDdlPlan` → CLI builds `DdlPlan` →
  `DriverSession::execute_ddl_plan` (Postgres real; others InvalidRequest)
- Unit: `ddl_add_column_review_emits_execute_ddl_plan`

## Commands

```bash
cargo test -p tablerock-tui --lib ddl_add_column
cargo test -p tablerock-cli --lib
cargo test -p tablerock-engine --lib
```

## Residual

- Drop column/index/constraint action entries in the action bar
- Structure panel quick-actions wired to same review dialog
