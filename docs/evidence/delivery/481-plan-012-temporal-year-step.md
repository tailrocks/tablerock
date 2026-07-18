# Plan 012 residual — IncYear / DecYear temporal step

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| step_year via ±12 months | done |
| Leap-day clamp (Feb 29 → Feb 28) | done |
| Actions Year+ / Year- | done |
| Unit test | done |

## Decision

Year step reuses month arithmetic so leap days clamp consistently with Mon±.
Presentation-only while editing; commit still stages the buffer.

## Evidence

```text
cargo test -p tablerock-tui --lib temporal_step_year
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for year step
