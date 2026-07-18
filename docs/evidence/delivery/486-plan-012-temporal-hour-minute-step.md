# Plan 012 residual — IncHour/DecHour / IncMinute/DecMinute

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| step_hour with day carry | done |
| step_minute with hour/day carry | done |
| Date-only injects midnight | done |
| Preserve timezone suffix | done |
| Actions Hour± / Min± | done |
| Unit test | done |

## Decision

Temporal editors need time-of-day steps beyond day/month/year. Hour and minute
steps carry through midnight; date-only buffers start at 00:00:00. Timezone
suffix (e.g. `Z`) is preserved.

## Evidence

```text
cargo test -p tablerock-tui --lib temporal_step_hour_and_minute
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for hour/minute temporal steps
