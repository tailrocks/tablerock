# Plan 012 residual — step cursor across visible columns

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `step_cursor_visible_column` | done |
| `is_column_visible` helper | done |
| Left/Right scroll uses visible step | done |
| Skips hidden layout columns | done |
| Edge no-op | done |
| Unit test | done |

## Decision

After Solo/ColHideE/ColPk, physical cursor Left/Right still walked hidden
columns. Horizontal scroll now steps the visible layout set so navigation
matches what the grid paints.

## Evidence

```text
cargo test -p tablerock-tui --lib step_cursor_visible
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for visible-column cursor step
