# Plan 012 residual — GoToNextStaged / GoToPrevStaged

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| go_to_next_staged wrap | done |
| go_to_prev_staged wrap | done |
| Cell edits + delete rows | done |
| Fail closed when empty | done |
| Actions NextStg / PrevStg | done |
| Unit test | done |

## Decision

Reviewing many staged cells needs cursor jump without the ShowStaged panel.
Targets sort by (row, col); wrap at ends. Deletes land on column 0 of the row.

## Evidence

```text
cargo test -p tablerock-tui --lib go_to_next_prev_staged
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for staged cursor jump
