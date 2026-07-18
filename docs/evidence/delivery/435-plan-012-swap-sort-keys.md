# Plan 012 residual — SwapSortKeys

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `swap_primary_secondary_sort` | done |
| Needs ≥2 keys | done |
| Tertiary keys untouched | done |
| Action SortSwap + rebrowse | done |
| Unit test | done |

## Decision

SortRot left-rotates the whole multi-key list. Operators often want only
primary↔secondary exchange without moving tertiary keys. SortSwap swaps
indices 0 and 1 only.

## Evidence

```text
cargo test -p tablerock-tui --lib swap_primary_secondary
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for primary/secondary swap
