# Plan 012 residual — FilterNotStartsWith / FilterNotEndsWith

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| FilterNotStartsWith NOT LIKE prefix | done |
| FilterNotEndsWith NOT LIKE suffix | done |
| Affix helper negate flag | done |
| Actions NStarts / NEnds | done |
| Unit test | done |

## Decision

Exclude prefix/suffix patterns use the same affix wrapping as Starts/Ends,
with notlike operator. Reuses residual 479 NotLike engine support.

## Evidence

```text
cargo test -p tablerock-tui --lib filter_starts_and_ends
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for NOT affix filters
