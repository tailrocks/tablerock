# Plan 013 residual — identity columns on status line

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| Editable grids show `pk col…` on status | done |
| Cap 4 names + `+N` overflow | done |
| Read-only still shows reason label | done |
| Unit test | done |

## Decision

Operators need to see which columns prove editability without opening
structure. Status line is the live surface; no I/O.

## Evidence

```text
cargo test -p tablerock-tui --lib recompute_editability
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for identity status projection
