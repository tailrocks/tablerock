# Plan 018 residual — CI first-row budgets + pg_dump matrix

Date: 2026-07-18

## What landed

`.github/workflows/checks.yml` `real-servers` job now runs:

1. `cargo test -p tablerock-engine --test performance_real -- --nocapture`
2. `apt-get install postgresql-client`
3. `cargo test -p tablerock-cli --test pg_dump_real -- --nocapture`

These use GitHub `ubuntu-latest` as the fixed-spec runner surface for
budgets and dump/restore (skips gracefully only if tools/images missing).

## Residual

- Multi-OS fixed-spec matrix publish (macOS runners optional)
- XCFramework/notarize still plan 019 operator STOP

## Commands

Workflow path: `.github/workflows/checks.yml` (push to main / dispatch)
