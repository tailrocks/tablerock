# Plan 012 residual — RotateSortRight

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `rotate_sort_keys_right` last → primary | done |
| Needs ≥2 keys | done |
| Inverse of left rotate on 2-key lists | done |
| Action SortRotR + rebrowse | done |
| Unit test | done |

## Decision

SortRot is left-rotate only. Symmetric right-rotate makes tertiary keys
primary without rebuilding the list or using SortSwap twice.

## Evidence

```text
cargo test -p tablerock-tui --lib invert_primary
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for right rotate
