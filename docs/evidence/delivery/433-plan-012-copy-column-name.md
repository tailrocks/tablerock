# Plan 012 residual — CopyColumnName

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyCursor column identifier OSC 52 | done |
| Empty columns no-op | done |
| Action CopyColN | done |
| Unit test | done |

## Decision

CopyCols emits all visible headers TSV. Operators often need the single
focused column identifier for SQL snippets. CopyColN copies `columns[cursor]`.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_column_names_emits
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for single column-name copy
