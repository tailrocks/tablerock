# Plan 012 residual — FilterStartsWith / FilterEndsWith

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| StartsWith → LIKE `value%` | done |
| EndsWith → LIKE `%value` | done |
| Skip wrap when value already has `%` | done |
| Null/empty fail closed | done |
| Actions Starts/Ends + rebrowse | done |
| Unit test | done |

## Decision

FilterLike wraps both sides (`%value%`). Prefix/suffix browsing is a
common residual without raw WHERE. Reuses the LIKE operator and plan
parameter path; no new FilterOperator variants.

## Evidence

```text
cargo test -p tablerock-tui --lib filter_starts_and_ends
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for affix LIKE filters
