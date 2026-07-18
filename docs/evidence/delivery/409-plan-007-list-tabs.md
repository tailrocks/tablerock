# Plan 007 residual — ListTabs inspector inventory

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `tabs_panel_text` with > * R p markers | done |
| Action ListTabs → inspector kind `tabs` | done |
| Unit test | done |

## Decision

Tab strip is space-limited. ListTabs dumps index + title + dirty/running/
preview glyphs so operators can GoToTab by name after scanning.

## Evidence

```text
cargo test -p tablerock-tui --lib tabs_panel
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for tab inventory
