# Plan 012 residual — InvertColumns visibility flip

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `invert_column_visibility` | done |
| At least one column remains visible | done |
| Action ColInv | done |
| Unit test via Solo → Invert | done |

## Decision

Invert is the complement of Solo for multi-column hide/show experiments
without enumerating toggles. Fail-open: if invert would hide all, first
layout entry stays visible.

## Evidence

```text
cargo test -p tablerock-tui --lib solo_cursor
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for invert visibility
