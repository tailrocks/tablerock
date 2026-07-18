# Plan 012 residual — CopyFiltersOnly

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyFiltersOnly OSC 52 filter chips only | done |
| Empty filters no-op | done |
| Does not include sort line | done |
| Action CopyFilt | done |
| Unit test | done |

## Decision

CopyBar joins sort + filter chip lines; CopySort is sort-only. Operators
often want filter provenance alone for tickets. CopyFilt emits
`filter_chip_bar()` only (typed chips, raw WHERE, page-local quick).

## Evidence

```text
cargo test -p tablerock-tui --lib copy_filters_only
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for filters-only copy
