# Plan 007 residual — GoToTab by title

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `select_tab_by_title` exact + unique prefix | done |
| Ambiguous prefix fail closed | done |
| Action GoTab + ConfirmDialog | done |
| Unit test | done |

## Decision

Mirrors GoToColumn resolution. Multi-tab sessions jump by name without
walking Next/Prev.

## Evidence

```text
cargo test -p tablerock-tui --lib select_tab_by_title
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for tab title jump
