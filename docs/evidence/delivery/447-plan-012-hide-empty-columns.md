# Plan 012 residual — HideEmptyColumns

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `hide_empty_resident_columns` | done |
| Null/Empty/Pending/empty text | done |
| Page-local only | done |
| At least one column remains | done |
| Action ColHideE | done |
| Unit test | done |

## Decision

Wide sparse pages often have all-null decorative columns. ColHideE hides
columns empty across the resident window without re-query. ColAll restores.

## Evidence

```text
cargo test -p tablerock-tui --lib hide_empty_resident
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for hide-empty columns
