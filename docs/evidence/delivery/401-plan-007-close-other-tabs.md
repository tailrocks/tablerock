# Plan 007 residual — CloseOtherTabs

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `close_other_tabs` keeps active only | done |
| Dirty other tabs → NeedsConfirm (fail closed) | done |
| Action CloseOthers | done |
| Unit test | done |

## Decision

Product: no silent drop of staged work. CloseOthers refuses when any
non-active tab is dirty; operator must CloseTab/Discard first.

## Evidence

```text
cargo test -p tablerock-tui --lib close_other
cargo test -p tablerock-tui --lib
```

## Remaining work

- Bulk discard-or-close flow for multiple dirty others (optional)
