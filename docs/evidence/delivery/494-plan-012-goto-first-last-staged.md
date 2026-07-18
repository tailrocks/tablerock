# Plan 012 residual — GoToFirstStaged / GoToLastStaged

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| go_to_first_staged | done |
| go_to_last_staged | done |
| Fail closed when empty | done |
| Actions FirstStg / LastStg | done |
| Unit test | done |

## Decision

Next/Prev cycle through staged targets; first/last jump straight to the ends
without cycling. Same target set as residual 489 (cell edits + deletes).

## Evidence

```text
cargo test -p tablerock-tui --lib go_to_next_prev_staged
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for first/last staged jump
