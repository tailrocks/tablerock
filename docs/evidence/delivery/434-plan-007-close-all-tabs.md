# Plan 007 residual — CloseAllTabs

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `close_all_tabs` clears strip | done |
| Dirty fail-closed confirm | done |
| Empty strip no-op | done |
| Action CloseAll | done |
| Unit test | done |

## Decision

CloseOthers keeps the active tab. Session reset needs empty strip without
closing one-by-one. CloseAll removes every tab under the same dirty gate.

## Evidence

```text
cargo test -p tablerock-tui --lib close_all_tabs
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for close-all
