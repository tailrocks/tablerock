# Plan 012 residual — CopyCellEditBuffer / CopyCellEditOriginal

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyCellEditBuffer current buffer | done |
| CopyCellEditOriginal pre-edit text | done |
| Fail closed when not editing | done |
| Actions CopyEdBuf / CopyEdOrig | done |
| Unit test | done |

## Decision

While editing, operators need the live buffer and the original value without
cancelling the session. Discrete copy actions fail closed when no edit is open
(distinct from CopyCell on committed grid projection).

## Evidence

```text
cargo test -p tablerock-tui --lib copy_cell_edit_buffer_and_original
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for edit-buffer copy
