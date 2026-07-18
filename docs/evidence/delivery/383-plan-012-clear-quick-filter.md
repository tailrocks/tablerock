# Plan 012 residual — ClearQuickFilter

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `ActionId::ClearQuickFilter` | done |
| Clears page-local filter without dialog | done |
| No server I/O | done |
| No-op when already empty | done |
| Unit test | done |

## Decision

EditQuickFilter still owns typed entry. Clear is a one-shot for operators
who set a long page-local needle and want it gone without paste-empty.

## Evidence

```text
cargo test -p tablerock-tui --lib edit_quick_filter
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for clear path
