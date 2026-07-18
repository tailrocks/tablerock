# Plan 012 residual — CloseAllPreviewTabs

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| close_all_preview_tabs keeps durable | done |
| No-op when no previews | done |
| Selection clamps after close | done |
| Action ClosePrevs | done |
| Unit test | done |

## Decision

Preview peeks pile up during catalog browse. One action clears them without
touching pinned/dirty durable tabs. Dirty always promotes out of preview first.

## Evidence

```text
cargo test -p tablerock-tui --lib close_all_preview_tabs
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for close-all-preview
