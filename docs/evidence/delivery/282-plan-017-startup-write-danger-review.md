# Plan 017 residual — Startup Write/Dangerous review UI

Date: 2026-07-18

## What landed

### Editor safety prefixes
- Startup SQL lines accept:
  - default → `ReadOnly` (auto-run)
  - `!write ` / `!w ` → `Write`
  - `!danger ` / `!dangerous ` / `!d ` → `Dangerous`
- Unit: `startup_sql_parses_write_and_danger_prefixes`

### Connect-time review
- `open_described_session` returns `startup_pending` for
  `review_required` actions
- `ConnectOk.startup_pending` opens `ConfirmDialog::StartupReview`
- Paste `RUN` + Submit → `Effect::ExecuteStartupReviewed`
- `DriverSession::execute_startup_authorized` on PG/CH/Redis/SessionSlot
  (timeout-bounded; no safety re-check — review is the gate)
- `StartupReviewDone` updates session status summary

### Still skipped without RUN
- Auto-run path still only executes ReadOnly
  (`SkippedNeedsReview` for Write/Dangerous)

## Commands

```bash
cargo test -p tablerock-tui startup
cargo check -p tablerock-cli -p tablerock-engine
```

## Residual

- DDL structure-panel quick actions
- Full pg_dump/pg_restore real-server matrix when CI has client binaries
