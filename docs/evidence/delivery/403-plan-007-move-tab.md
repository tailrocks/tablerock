# Plan 007 residual — MoveTabLeft / MoveTabRight

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `move_active_tab_left` / `move_active_tab_right` | done |
| Selection follows moved tab | done |
| End stops (no wrap for reorder) | done |
| Actions TabL / TabR | done |
| Unit test | done |

## Decision

Tab strip reorder is independent of Next/Prev selection wrap. Ends fail
closed (no-op) so order stays intentional.

## Evidence

```text
cargo test -p tablerock-tui --lib move_active_tab
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for tab reorder
