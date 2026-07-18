# Plan 012 residual — HalfPageUp/Down + CopyDatabaseIdent

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `half_page_step_rows` (page/2, min 1) | done |
| HalfPageUp / HalfPageDown actions | done |
| Shared `page_jump_rows` helper | done |
| CopyDatabaseIdent SQL-quoted | done |
| Actions HalfUp/HalfDn / CopyDbQ | done |
| Unit tests | done |

## Decision

Full-page jumps can overshoot dense reviews. Half-page uses the same
resident step basis (capped) halved with min 1. CopyDbQ quotes the active
database for `USE`/identifier paste (hostile names).

## Evidence

```text
cargo test -p tablerock-tui --lib page_step_and_step_cursor
cargo test -p tablerock-tui --lib page_up_down_jump
cargo test -p tablerock-tui --lib copy_session_id_and_engine
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for half-page jump / database ident copy
