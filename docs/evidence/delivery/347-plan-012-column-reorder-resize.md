# Plan 012 residual — column reorder and resize

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `move_cursor_column` (±1 in layout order) | done |
| `adjust_cursor_column_width` (clamp 4..=64) | done |
| VirtualGrid render uses `visible_columns` + layout widths | done |
| Toolbar: ColL / ColR / Col- / Col+ / ColVis / ColRst / ColSave | done |
| Layout JSON still persists order, visibility, width | done |
| Unit test reorder + width clamp | done |

## Decision

Physical page matrix order stays fixed (engine projection). Display order and
width live only in `column_layout`, so reorder never remaps cell arenas.
Hidden columns stay in layout for SaveColumns restore.

## Evidence

```text
cargo test -p tablerock-tui --lib move_and_resize
cargo check -p tablerock-tui
```

## Remaining work

- Drag-to-reorder (mouse) if TermRock exposes header drag later
