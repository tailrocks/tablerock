# Plan 012 residual — reveal cursor column after GoToColumn

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `reveal_cursor_column` sets viewport_col = cursor_col | done |
| GoToColumn submit calls reveal | done |
| Unit tests | done |

## Decision

Named column jumps on wide grids left-align the target in the horizontal
viewport so the operator sees the jumped column immediately.

## Evidence

```text
cargo test -p tablerock-tui --lib reveal_cursor
cargo test -p tablerock-tui --lib go_to_column
cargo test -p tablerock-tui --lib
```

## Remaining work

- Soft scroll window (keep N columns of context) optional
