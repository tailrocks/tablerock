# Plan 012 residual — CopyCursorPosition

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyCursorPosition `row,col (name)` | done |
| Empty columns no-op | done |
| Action CopyPos | done |
| Unit test | done |

## Decision

Debug tickets need absolute row + physical column + name without reading
the status line. CopyPos emits `row,col (name)`.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_cursor_position
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for cursor position copy
