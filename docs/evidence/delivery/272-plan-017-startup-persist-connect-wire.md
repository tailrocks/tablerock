# Plan 017 residual — startup persist + connect wire; SSH property CHECK fix

Date: 2026-07-18

## What landed

### Persistence migration `0011`
- Rebuild `saved_profile_properties` with `property BETWEEN 1 AND 16` and
  `ordinal BETWEEN 0 AND 15` (SSH fields 11–16 were unpersisted under CHECK 1–10)
- New `saved_profile_startup_actions` table

### Profile aggregate
- `startup_actions: StartupActionSet` (default empty)
- `with_startup_actions` / `startup_actions()` accessors
- Create/update/load round-trip

### Connect path
- `ConnectionDraft.startup_actions`
- `aggregate_to_draft` / `draft_to_aggregate` carry the set
- `open_described_session(..., is_reconnect)` runs
  `run_postgres_startup_actions` after PG connect (reconnect uses reconnect filter)
- Partial failure does not abort connect

## Commands

```bash
cargo test -p tablerock-persistence --test profile_create startup_actions
cargo test -p tablerock-persistence --tests
cargo test -p tablerock-cli --lib
cargo test -p tablerock-engine --test startup_actions_real
```

## Residual

- TUI editor for startup SQL list
- CH/Redis startup executors
- Surface StartupRunReport in connect UI
