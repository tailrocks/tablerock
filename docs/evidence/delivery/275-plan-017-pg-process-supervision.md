# Plan 017 residual — pg_dump/pg_restore process supervision

Date: 2026-07-18

## What landed

- `pg_restore_argv` (password never on argv; `--no-password`)
- `crates/tablerock-cli/src/pg_process.rs`
  - `run_pg_dump` / `run_pg_restore` via direct `tokio::process::Command`
  - `PGPASSWORD` env only when password provided
  - Cancel via watch channel → `start_kill` + remove incomplete dump file
  - `validate_dump_path`
- Unit tests:
  - cancel kills `sleep` stand-in and removes partial output
  - `true` succeeds
  - argv secret hygiene for dump/restore builders
  - path validation

## Commands

```bash
cargo test -p tablerock-cli --lib pg_process
cargo test -p tablerock-cli --lib tool_discovery
```

## Version matrix gap (honest)

Host at evidence time: **no `pg_dump`/`pg_restore` on PATH**. Real-server
dump/restore against containerized Postgres remains **local-only** when the
operator installs client tools (same pattern as `performance_real` CI gap).
Discovery still returns explicit `Missing` without clients.

## Residual

- TUI/action wiring for dump/restore
- Real Docker + installed client matrix when CI runners ship client packages
