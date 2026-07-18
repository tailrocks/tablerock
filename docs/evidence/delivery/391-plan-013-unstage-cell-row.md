# Plan 013 residual — per-change UnstageCell / UnstageRow

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `discard_cell_edit` / `discard_delete` / `discard_row_stages` | done |
| `unstage_cursor_cell` / `unstage_cursor_row` | done |
| Actions UnstgCell / UnstgRow | done |
| Cancels open cell edit on same target | done |
| Dirty clears when drafts empty | done |
| Unit tests | done |

## Decision

Product requires per-change discard from the staged view. Cursor-targeted
unstage is the keyboard-first path: one cell or whole row without DiscardAll.
Insert drafts still use DropIns / Undo.

## Evidence

```text
cargo test -p tablerock-tui --lib discard_cell
cargo test -p tablerock-tui --lib unstage_cursor
cargo test -p tablerock-tui --lib
```

## Remaining work

- Unstage by picking a line in ShowStaged panel (optional)
