# Plan 012 residual — fit column width to content

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `fit_cursor_column` from header + resident cells | done |
| `fit_all_visible_columns` | done |
| Bounds 4..=64; no-op when already fitted | done |
| Actions ColFit / ColFitA in toolbar | done |
| Unit test fit single + fit all after widen | done |

## Decision

Fit uses only the resident page matrix (no extra I/O). Width is character
count of `ProjectedCell::display()`, clamped to layout bounds. Persist via
existing SaveColumns JSON.

## Evidence

```text
cargo test -p tablerock-tui --lib fit_column
cargo check -p tablerock-tui
```

## Remaining work

- Mouse drag resize if TermRock VirtualGrid exposes it
