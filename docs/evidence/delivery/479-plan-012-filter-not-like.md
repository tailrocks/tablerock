# Plan 012 residual — FilterNotLike / FilterINotLike

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| FilterOperator::NotLike / NotILike | done |
| Parameterized browse_plan SQL | done |
| effects.rs operator map | done |
| filters_where_sql presentation | done |
| Actions NLike / NILike | done |
| Unit tests | done |

## Decision

Exclude-by-pattern is a first-class typed filter, not raw WHERE. Patterns
bind as `$n` parameters; NOT LIKE / NOT ILIKE never concatenate cell text.

## Evidence

```text
cargo test -p tablerock-engine --lib not_like_operators
cargo test -p tablerock-tui --lib filter_like
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for NOT (I)LIKE filters
