# Plan 012 residual — ResetColumnWidths

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `reset_column_widths` → width 12 | done |
| Keeps order + visibility | done |
| No-op when already default | done |
| Action ColRstW | done |
| Unit test | done |

## Decision

ColRst rebuilds layout (order, visibility, widths). After Fit/Col± operators
often want default widths without undoing Solo/hide/reorder. ColRstW is
widths-only.

## Evidence

```text
cargo test -p tablerock-tui --lib reset_column_widths
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for width-only reset
