# Plan 007 residual — CloseTabsToLeft / CloseTabsToRight

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `close_tabs_to_right` truncate | done |
| `close_tabs_to_left` drain | done |
| Dirty fail-closed confirm | done |
| Empty when nothing to close | done |
| Actions CloseRight / CloseLeft | done |
| Unit test | done |

## Decision

CloseOthers keeps only the active tab. Operators often want IDE-style
close-to-right / close-to-left while preserving the other side of the
strip. Same dirty gate as CloseOthers.

## Evidence

```text
cargo test -p tablerock-tui --lib close_tabs_to_left_and_right
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for directional bulk close
