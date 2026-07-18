# Plan 012 residual — CopyCursorStagedDiff

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| Copy `original → staged` for cursor cell | done |
| Fail closed if not staged | done |
| Action CopyStgDiff | done |
| Unit test | done |

## Decision

Support tickets and reviews need the before/after pair without opening the
staged panel or inspector. Distinct from live edit-buffer copy (488).

## Evidence

```text
cargo test -p tablerock-tui --lib copy_cursor_staged_diff
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for staged-diff copy
