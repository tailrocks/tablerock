# Plan 012 residual — CommitCellEdit / RestoreCellEdit

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CommitCellEdit stages draft + dirty | done |
| RestoreCellEdit resets buffer, stays editing | done |
| No-op when not editing | done |
| Actions CommitEd / RestoreEd | done |
| Unit tests (grid + update) | done |

## Decision

Content Enter already commits via Activate when cell_edit is open. Palette
CommitEd/RestoreEd expose commit and original-buffer restore for action
discovery and keymap binding without leaving Actions focus.

## Evidence

```text
cargo test -p tablerock-tui --lib restore_cell_edit_buffer
cargo test -p tablerock-tui --lib commit_and_restore_cell_edit
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for commit/restore cell-edit actions
