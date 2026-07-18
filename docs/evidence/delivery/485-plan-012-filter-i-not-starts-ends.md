# Plan 012 residual — FilterINotStartsWith / FilterINotEndsWith

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| FilterINotStartsWith NOT ILIKE prefix | done |
| FilterINotEndsWith NOT ILIKE suffix | done |
| Reuses affix helper + NotILike engine | done |
| Actions INStarts / INEnds | done |
| Unit test | done |

## Decision

Case-insensitive exclude-by-prefix/suffix completes the affix matrix
(Starts/Ends/IStarts/IEnds/NStarts/NEnds). Patterns bind as parameters.

## Evidence

```text
cargo test -p tablerock-tui --lib filter_starts_and_ends
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for NOT ILIKE affix filters
