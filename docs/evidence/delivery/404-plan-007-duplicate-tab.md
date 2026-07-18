# Plan 007 residual — DuplicateTab shallow clone

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `duplicate_active_tab` | done |
| Title gets ` (copy)` suffix | done |
| Drafts and cell_edit cleared | done |
| Dirty/running/preview reset | done |
| Action DupTab | done |
| Unit test | done |

## Decision

Shallow clone of resident grid/editor state for side-by-side compare. Mutation
drafts intentionally do not clone — dual apply of the same staged set is a
safety hazard.

## Evidence

```text
cargo test -p tablerock-tui --lib duplicate_active
cargo test -p tablerock-tui --lib
```

## Remaining work

- Optional deep-clone of drafts behind explicit confirm (not default)
