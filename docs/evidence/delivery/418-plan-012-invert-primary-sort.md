# Plan 012 residual — InvertPrimarySort

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `invert_primary_sort` Asc↔Desc | done |
| No-op when sort empty | done |
| Action SortInv + rebrowse | done |
| Unit test | done |

## Decision

CycleSort rotates None→Asc→Desc→None and reorders keys. Invert only flips
the primary direction without clearing multi-column order.

## Evidence

```text
cargo test -p tablerock-tui --lib invert_primary
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for primary sort invert
