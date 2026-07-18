# Plan 009 residual — CopyNotices to clipboard

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `ActionId::CopyNotices` | done |
| OSC 52 via CopyToClipboard | done |
| No-op when history empty | done |
| Status reports byte count | done |

## Decision

ShowNotices is view-only. CopyNotices pastes the same bounded redacted panel
text so operators can file tickets without retyping RAISE NOTICE lines.

## Evidence

```text
cargo test -p tablerock-tui --lib
cargo check -p tablerock-tui
```

## Remaining work

- None material for notice copy
