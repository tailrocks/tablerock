# Plan 012 residual — CopyColumnNames

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| Action CopyCols | done |
| Visible columns only (layout-aware) | done |
| Tab-separated names for paste into SELECT | done |
| Unit test with ColSolo | done |

## Decision

Header list is often needed for SQL authoring. Visibility follows column
layout so Solo/hide is respected.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_column_names
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for column name copy
