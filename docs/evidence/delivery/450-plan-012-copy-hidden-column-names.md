# Plan 012 residual — CopyHiddenColumnNames

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `hidden_columns` helper | done |
| CopyHiddenColumnNames OSC 52 TSV | done |
| No hidden → no-op | done |
| Action CopyHid | done |
| Unit test | done |

## Decision

CopyCols emits visible headers. After Solo/ColHideE operators need the
list of hidden names for restore notes. CopyHid emits hidden layout
names tab-separated.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_hidden_column_names
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for hidden column name copy
