# Plan 012 residual — ReverseSortKeys

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `reverse_sort_keys` full list | done |
| Needs ≥2 keys | done |
| Directions preserved | done |
| Action SortRev + rebrowse | done |
| Unit test | done |

## Decision

SortRot/SortRotR cycle keys; SortSwap only touches primary/secondary.
Full reverse of a 3+ key ORDER BY is a common multi-key residual for
least-significant-first reading. SortRev reverses the key vector without
flipping Asc/Desc (use SortInvA for that).

## Evidence

```text
cargo test -p tablerock-tui --lib reverse_sort_keys
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for reverse sort keys
