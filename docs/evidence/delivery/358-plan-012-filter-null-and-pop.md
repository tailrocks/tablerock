# Plan 012 residual — IS NULL / IS NOT NULL filters and chip pop

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `FilterIsNull` / `FilterIsNotNull` on cursor column | done |
| `RemoveLastFilter` pops newest chip + rebrowse | done |
| `RemoveColumnFilters` clears chips for cursor column | done |
| Toolbar: IsNull / NotNull / PopFilt / ClrColF | done |
| Engine ops `isnull` / `isnotnull` already mapped | done (prior) |
| Unit tests model + reducer | done |

## Decision

Null filters need no value (FilterOperator::needs_value false). Pop is
LIFO over the chip list so operators can undo the last AddFilter without
clearing the whole bar.

## Evidence

```text
cargo test -p tablerock-tui --lib filter_null
cargo test -p tablerock-tui --lib remove_last_and_column
cargo check -p tablerock-tui
```

## Remaining work

- Click-to-remove individual chips (optional)
