# Plan 012 residual — Home/End visible columns + PageColumnLeft/Right

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| HomeCursor first visible column | done |
| EndCursor last visible column | done |
| PageColumnLeft half visible set | done |
| PageColumnRight half visible set | done |
| Skip hidden columns | done |
| Unit tests (grid + update) | done |

## Decision

After ColHideE/solo, Home/End must not re-enter hidden physical columns 0/last.
Horizontal page jump uses half of the visible column count (capped 1..=10),
matching HalfPage row navigation.

## Evidence

```text
cargo test -p tablerock-tui --lib home_end_cursor_skip_hidden
cargo test -p tablerock-tui --lib step_cursor_visible_column_by
cargo test -p tablerock-tui --lib page_column_left_right
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for visible column page nav
