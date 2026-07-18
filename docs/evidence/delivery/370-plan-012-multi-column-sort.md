# Plan 012 residual — multi-column sort Push/Pop + chip bar

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `push_sort_column` appends secondary / cycles in place | done |
| `pop_sort_key` removes least-significant key | done |
| `sort_chip_bar` primary-first ORDER BY list | done |
| Actions Sort+ / Sort- | done |
| Workbench ⇅ sort bar above filter bar | done |
| CycleSort still promotes to primary | done |
| Unit test | done |

## Decision

CycleSort remains the one-key primary cycle. PushSort builds multi-column
ORDER BY without reordering existing keys; PopSort peels the last key.
Chip bar is presentation-only (glyph ⇅); server re-browse still goes through
`BrowsePlan` quoted sort keys.

## Evidence

```text
cargo test -p tablerock-tui --lib push_sort
cargo test -p tablerock-tui --lib
cargo check -p tablerock-tui
```

## Remaining work

- Interactive chip focus to reorder sort keys (optional)
