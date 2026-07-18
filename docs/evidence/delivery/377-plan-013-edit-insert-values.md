# Plan 013 residual — EditInsert values dialog

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `replace_insert_values` / `last_insert` on drafts | done |
| Action EditInsert opens dialog for last insert | done |
| `col=value` line buffer parse | done |
| Empty value → NULL at plan build (existing heuristic) | done |
| Unit test | done |

## Decision

Blank/duplicate insert staging (371) needs a value entry path before Review.
Paste-oriented `col=value` lines match other confirm buffers (filters, Redis
stage) and avoid inventing a multi-field form. Targets the last staged insert
only; Undo still peels whole inserts.

## Evidence

```text
cargo test -p tablerock-tui --lib replace_insert
cargo test -p tablerock-tui --lib
```

## Remaining work

- Virtual inserted-row viewport (optional)
