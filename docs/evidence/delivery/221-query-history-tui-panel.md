# Query history TUI panel

Date: 2026-07-18

## Checkpoint

Plan 011 step 4 (TUI). History panel load via `Effect::LoadHistory`,
restore into editor via `RestoreHistory` (no auto-execute). Successful
SQL stream completion best-effort appends through `Effect::AppendHistory`
using workbench retention policy (`full` default).

## Decision

- History entries restore statement text only; never re-run.
- Append uses last `run_text()` after `GridStreamComplete` (best-effort;
  incomplete if editor mutated mid-stream).

## Evidence

- `update::tests::history_load_and_restore_into_editor_without_auto_run`
- `cargo test -p tablerock-tui -p tablerock-cli --lib`
- Persistence: evidence 220

## Remaining (plan 011)

- Saved queries + `.sql` file open/save/atomic write
- Intent-only session restoration
- History search UI field / Up-Down selection keys
