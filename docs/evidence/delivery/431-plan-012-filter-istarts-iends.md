# Plan 012 residual — FilterIStartsWith / FilterIEndsWith

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| IStarts → ILIKE `value%` | done |
| IEnds → ILIKE `%value` | done |
| Shares affix helper with LIKE path | done |
| Actions IStarts / IEnds | done |
| Unit test | done |

## Decision

Starts/Ends use LIKE. Case-insensitive prefix/suffix browsing is common on
text columns without typing raw WHERE. Reuses ILIKE operator + affix wrap.

## Evidence

```text
cargo test -p tablerock-tui --lib filter_istarts_and_iends
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for ILIKE affix filters
