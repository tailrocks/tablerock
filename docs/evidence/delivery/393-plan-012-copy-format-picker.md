# Plan 012 residual — CopyPick format picker dialog

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `ConfirmDialog::CopyPick` | done |
| Action CopyPick | done |
| Parse `scope format` (row\|loaded + six formats) | done |
| Bare format defaults to row | done |
| Unit tests parse + dialog → OSC 52 | done |

## Decision

Product asks for a format picker; dedicated CopyCsv/CopyRow* actions remain
shortcuts. CopyPick is the typed dialog path for one-shot scope+format
selection without growing the action bar further.

## Evidence

```text
cargo test -p tablerock-tui --lib parse_copy_pick
cargo test -p tablerock-tui --lib copy_pick
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for TUI format picker
