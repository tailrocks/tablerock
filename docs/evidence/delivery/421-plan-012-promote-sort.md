# Plan 012 residual — PromoteSort

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `promote_sort_column` no direction cycle | done |
| Already-primary no-op | done |
| Secondary keeps direction | done |
| Absent inserts Asc primary | done |
| Action SortPri + rebrowse | done |
| Unit test | done |

## Decision

CycleSort both promotes and cycles direction (and can clear). PushSort
builds multi-key without promote. SortPri only reorders: promote cursor
column to primary ORDER BY while preserving its current direction, or
insert Asc when absent. Complements SortRot (global left rotate).

## Evidence

```text
cargo test -p tablerock-tui --lib promote_sort
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for promote-sort
