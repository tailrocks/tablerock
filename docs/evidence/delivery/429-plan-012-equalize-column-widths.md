# Plan 012 residual — EqualizeColumnWidths

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `equalize_visible_column_widths` | done |
| Uses cursor column width | done |
| Hidden columns untouched | done |
| No-op when already equal | done |
| Action ColEqW | done |
| Unit test | done |

## Decision

FitAll measures each column independently. Operators often want a uniform
width from the focused column for dense tables. ColEqW copies cursor width
to every visible layout entry.

## Evidence

```text
cargo test -p tablerock-tui --lib equalize_visible
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for equalize widths
