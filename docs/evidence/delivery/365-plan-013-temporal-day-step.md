# Plan 013 residual — temporal Day± step while editing

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `CellEditSession::step_day` | done |
| Preserves `T…` / space time suffixes | done |
| Month/year boundary via proleptic Gregorian | done |
| Invalid date text fail closed | done |
| Actions Day+ / Day- | done |
| Unit test | done |

## Decision

Day step is the lightweight alternative to a full calendar month widget.
Today/Now still stamp absolute values; Day± adjusts the date portion only.

## Evidence

```text
cargo test -p tablerock-tui --lib temporal_step
cargo check -p tablerock-tui
```

## Remaining work

- Full calendar month picker widget (optional)
