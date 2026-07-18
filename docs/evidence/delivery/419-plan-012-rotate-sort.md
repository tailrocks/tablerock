# Plan 012 residual — RotateSort multi-key order

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `rotate_sort_keys` left-rotate | done |
| Needs ≥2 keys | done |
| Action SortRot + rebrowse | done |
| Unit test | done |

## Decision

Promote secondary ORDER BY to primary without rebuilding the list from
scratch. Complements PushSort/PopSort/SortInv.

## Evidence

```text
cargo test -p tablerock-tui --lib invert_primary
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for sort rotation
