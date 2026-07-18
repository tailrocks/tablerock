# Plan 013 residual — CopyStaged inventory to clipboard

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `ActionId::CopyStaged` | done |
| OSC 52 of `staged_panel_text` | done |
| No-op when drafts empty | done |
| Status byte count | done |

## Decision

ShowStaged is the inventory view; CopyStaged pastes the same glyph-marked
text for external review notes without opening Review SQL.

## Evidence

```text
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for staged copy
