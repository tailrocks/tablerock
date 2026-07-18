# Plan 012 residual — FilterByLocator

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| Action FiltLoc | done |
| Eq chips for each identity column of cursor row | done |
| NULL identity → isnull chip | done |
| Rebrowse | done |
| Unit test | done |

## Decision

Narrow the grid to the current identity row without typing filters.
Uses the same locator facts as mutations. Complements FollowForeignKey.

## Evidence

```text
cargo test -p tablerock-tui --lib filter_by_locator
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for locator filter
