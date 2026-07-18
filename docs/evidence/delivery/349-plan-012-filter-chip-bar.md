# Plan 012 residual — filter chip bar

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `filter_chip_bar()` builds `[col op val]` tokens | done |
| Raw WHERE + page-local chips included | done |
| Cap 12 chips + "+N more" | done |
| Workbench renders ▣ bar above VirtualGrid when present | done |
| Status line shows filter snippets (not only count) | done |
| Unit tests for bar + empty | done |

## Decision

Chip bar is presentation-only over existing `GridFilterChip` / `raw_where` /
`quick_filter` state. Glyph `▣` marks the bar without relying on color.
Editing chips remains via AddFilter / ClearFilters / presets.

## Evidence

```text
cargo test -p tablerock-tui --lib filter_chip
cargo test -p tablerock-tui --lib cycle_sort
cargo check -p tablerock-tui
```

## Remaining work

- Interactive chip focus/remove (optional)
