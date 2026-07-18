# Plan 013 residual — draft overlay markers in VirtualGrid

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `cell_display_at` staged value overlay (`· value`) | done |
| Deleted row marker (`−`) on every cell | done |
| Live cell-edit buffer (`✎ buffer`) | done |
| VirtualGrid paint uses `cell_display_at` | done |
| Unstaged cells on modified row stay original | done |
| Unit test | done |

## Decision

Product requires staged state visible without color alone. Presentation is
pure (no I/O): live edit buffer wins, then delete marker, then staged cell
overlay, else base `ProjectedCell::display`. Insert drafts remain status/
review only until an inserted-row viewport residual lands.

## Evidence

```text
cargo test -p tablerock-tui --lib begin_and_commit
cargo test -p tablerock-tui --lib
```

## Remaining work

- Virtual inserted-row viewport for InsRow drafts (optional; evidence 371)
