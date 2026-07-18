# Plan 017 residual — pg_dump/pg_restore real-server matrix + TUI

Date: 2026-07-18

## What landed

### Client tools
- Host: `pg_dump` / `pg_restore` **18.4** via Homebrew `libpq` keg
  (`/opt/homebrew/opt/libpq/bin/`)
- Discovery still accepts PATH or explicit path; test falls back to keg paths

### Process + format
- `pg_dump_argv` uses **`-Fc`** (custom format) so `pg_restore` can load
- Password remains env-only (`PGPASSWORD`); never argv

### Real Docker matrix
- `crates/tablerock-cli/tests/pg_dump_real.rs`
  - Postgres **18.4-alpine** container
  - dump non-empty archive
  - restore into fresh database on same server
  - skips cleanly when clients missing

### TUI
- `ActionId::PgDump` / `PgRestore` → `ConfirmDialog::PgTool` (path paste)
- `Effect::RunPgDump` / `RunPgRestore` → supervised runner
- `PgToolDone` updates session/grid status
- Unit: `pg_dump_action_opens_confirm_and_emits_run_effect`

## Commands

```bash
export PATH="/opt/homebrew/opt/libpq/bin:$PATH"
cargo test -p tablerock-tui pg_dump
cargo test -p tablerock-cli --test pg_dump_real
cargo check -p tablerock-cli
```

## Residual

- Plan 017 residuals closed for software-side dump/restore
- CI still needs client packages on runners for non-skip runs
