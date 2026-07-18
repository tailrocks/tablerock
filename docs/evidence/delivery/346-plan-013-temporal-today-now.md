# Plan 013 residual — temporal Today / Now staging stamps

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `CellEditSession::set_today` / `set_now` for Temporal kind | done |
| `ActionId::SetToday` / `SetNow` action bar | done |
| Stamps pass existing temporal validation heuristics | done |
| Inspector hint for temporal cells | done |
| No new calendar crate dependency | done |

## Decision

Full interactive calendar widget remains optional polish. Today/Now stamps
use proleptic Gregorian from UNIX time (UTC `Z` for now). Operators can
still type arbitrary temporal text.

## Evidence

```text
cargo test -p tablerock-tui --lib temporal_set
cargo test -p tablerock-tui --lib inspector
cargo check -p tablerock-cli
```

## Remaining work

- Optional full calendar widget (month grid)
