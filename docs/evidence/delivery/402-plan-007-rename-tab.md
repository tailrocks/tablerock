# Plan 007 residual — RenameTab dialog

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `ConfirmDialog::RenameTab` | done |
| 1–128 chars, no control characters | done |
| Promotes preview → durable tab | done |
| Action RenTab | done |
| Unit test | done |

## Decision

Operators rename result/SQL tabs for multi-tab sessions without closing.
Empty or control-laden titles fail closed (dialog stays open).

## Evidence

```text
cargo test -p tablerock-tui --lib rename_tab
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for tab rename
