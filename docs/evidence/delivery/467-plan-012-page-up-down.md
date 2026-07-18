# Plan 012 residual — PageUp / PageDown

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `page_step_rows` (resident size, cap 100) | done |
| `step_cursor_row` helper | done |
| PageUp subtracts step (floor 0) | done |
| PageDown adds step (cap totals) | done |
| Uses jump_to_row (FetchPage when needed) | done |
| Actions PgUp / PgDn | done |
| Unit tests | done |

## Decision

Single-row scroll is insufficient for large pages. Page step uses the
resident `row_count` (min 1, max 100) so the jump matches the loaded
window size without inventing a separate preference.

## Evidence

```text
cargo test -p tablerock-tui --lib page_step_and_step_cursor
cargo test -p tablerock-tui --lib page_up_down_jump
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for page-up/down navigation
