# Plan 013 residual — temporal Month± + PickDate calendar

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `CellEditSession::step_month` (clamp day into month) | done |
| `month_calendar_text` / text month grid | done |
| Preserves `T…` / space time suffixes | done |
| Actions Mon+ / Mon- / Cal | done |
| `ConfirmDialog::PickDate` day or YYYY-MM-DD | done |
| Unit test | done |

## Decision

Month step clamps day into the target month (Jan 31 → Feb 28/29). PickDate
shows a text Su–Sa month grid and accepts day-of-month or full ISO date while
preserving the existing time suffix. No calendar crate dependency.

## Evidence

```text
cargo test -p tablerock-tui --lib temporal_step
cargo test -p tablerock-tui --lib
cargo check -p tablerock-tui
```

## Remaining work

- Interactive cursor-driven month navigation in dialog (optional polish)
