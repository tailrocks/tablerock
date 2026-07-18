# Plan 017 residual — PostgreSQL startup actions executor

Date: 2026-07-18

## What landed

- `run_postgres_startup_actions` in `tablerock-engine`
  - Auto-runs `StartupSafetyClass::ReadOnly` with per-action timeout
  - `Write` / `Dangerous` → `SkippedNeedsReview` (no free-SQL write on connect)
  - Continues after failure; report records partial outcomes
- Docker: SELECT succeeds, CREATE skipped, SELECT 1/0 fails; reconnect path
  still skips Write

## Commands

```bash
cargo test -p tablerock-engine startup
cargo test -p tablerock-engine --test startup_actions_real
```

## Residual

- Profile aggregate persistence of StartupActionSet
- Connect-path wiring after open_described_session
- CH/Redis startup where engine-correct
- Review UI for Write/Dangerous
