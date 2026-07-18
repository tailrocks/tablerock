# Plan 012 residual — SetYesterday / SetTomorrow

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| set_yesterday = today then day-1 | done |
| set_tomorrow = today then day+1 | done |
| Actions Yest / Tomor | done |
| Unit test | done |

## Decision

One-shot stamps for relative calendar dates without manual Day± from today.
Reuse day arithmetic so month boundaries stay consistent.

## Evidence

```text
cargo test -p tablerock-tui --lib set_today
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for yesterday/tomorrow stamps
