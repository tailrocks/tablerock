# Plan 012 residual — PromoteLastFilter

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `promote_last_filter` newest → front | done |
| Needs ≥2 chips | done |
| Action PromoFilt + rebrowse | done |
| Unit test | done |

## Decision

RevFilt reverses all chips. Operators often add a refined chip last but
want it read first in the bar / AND mental model. PromoFilt moves
filters[last] to index 0.

## Evidence

```text
cargo test -p tablerock-tui --lib remove_last_and_column
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for promote-last filter
