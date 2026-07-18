# Plan 012 residual — CopyFilterBar

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `ActionId::CopyFilterBar` | done |
| Copies sort + filter chip bar text | done |
| No-op when both empty | done |
| OSC 52 | done |

## Decision

Chip bars are presentation-only. CopyBar pastes the same text operators
see so they can share browse state without screenshots.

## Evidence

```text
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for filter bar copy
