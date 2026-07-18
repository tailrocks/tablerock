# Plan 013 residual — ShowStaged inspector inventory

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `staged_panel_text` lists inserts/edits/deletes | done |
| Action ShowStaged → inspector kind `staged` | done |
| Empty state message | done |
| Unit test | done |

## Decision

Status bar counts staged work; ShowStaged is the inventory view before
Review so operators can see every draft with glyphs (+ · −) without
opening the full SQL preview. Review remains the exact parameterized
statement list.

## Evidence

```text
cargo test -p tablerock-tui --lib staged_panel
cargo test -p tablerock-tui --lib
```

## Remaining work

- Per-item discard from panel (optional)
