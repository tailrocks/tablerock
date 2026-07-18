# Plan 012 residual — SnapCursorVisible

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `ensure_cursor_on_visible_column` | done |
| Integrated into ColPk / ColHideE | done |
| Action ColSnap | done |
| Unit test | done |

## Decision

After hide/solo layout changes the cursor may sit on a now-hidden physical
column. ColSnap (and automatic use from ColPk/ColHideE) moves it to the
first visible column and reveals the viewport.

## Evidence

```text
cargo test -p tablerock-tui --lib ensure_cursor_on_visible
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for cursor snap-to-visible
