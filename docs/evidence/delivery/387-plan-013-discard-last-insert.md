# Plan 013 residual — DiscardLastInsert

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `discard_last_insert` peels only last insert | done |
| Leaves cell edits / deletes intact | done |
| Action DropIns | done |
| Dirty tab clears when drafts empty | done |
| Unit test | done |

## Decision

Full DiscardStaged is heavy-handed for multi-insert experiments. DropIns is
the insert-only peel; Undo still walks the full stage stack.

## Evidence

```text
cargo test -p tablerock-tui --lib discard_last_insert
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for discard-last-insert
