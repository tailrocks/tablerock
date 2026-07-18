# Plan 012 residual — CopyColumnLayout

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyColumnLayout OSC 52 layout JSON | done |
| Empty columns no-op | done |
| Action ColJson | done |
| Unit test | done |

## Decision

ColSave persists layout via the actor. Operators also need the layout JSON
on the clipboard for tickets/debug without a round-trip through storage.
ColJson emits `layout_json()` for the active grid.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_column_layout
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for layout JSON copy
