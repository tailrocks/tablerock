# Plan 012 residual — ReverseFilters

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `reverse_filters` on ≥2 chips | done |
| Single/empty no-op | done |
| Action RevFilt + rebrowse | done |
| Unit test | done |

## Decision

AND chip order can matter for human reading of the chip bar and for
short-circuit planning mental models. RevFilt reverses the typed filter
list without changing operators/values.

## Evidence

```text
cargo test -p tablerock-tui --lib remove_last_and_column
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for reverse filters
