# Plan 012 residual — SetStartOfDay / SetNoon / SetEndOfDay

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| SetStartOfDay 00:00:00 | done |
| SetNoon 12:00:00 | done |
| SetEndOfDay 23:59:59 | done |
| Keep date + timezone | done |
| No-op when already set | done |
| Actions StartDay / Noon / EndDay | done |
| Unit test | done |

## Decision

Operators often need range bounds without typing times. One-shot stamps keep
the date (or today) and any timezone suffix while replacing the clock.

## Evidence

```text
cargo test -p tablerock-tui --lib temporal_set_start_noon_end
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for day-bound temporal stamps
