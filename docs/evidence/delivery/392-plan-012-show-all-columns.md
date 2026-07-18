# Plan 012 residual — ShowAllColumns keeps widths

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `show_all_columns` makes every column visible | done |
| Keeps widths and order (unlike ResetColumns) | done |
| Action ColAll | done |
| No-op when already all visible | done |
| Unit test with ColSolo → ColAll | done |

## Decision

ColRst wipes widths back to defaults. ColAll is the Solo complement: unhide
without losing Col± / ColFit work.

## Evidence

```text
cargo test -p tablerock-tui --lib solo_cursor
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for show-all visibility
