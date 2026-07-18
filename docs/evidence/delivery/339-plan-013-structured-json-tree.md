# Plan 013 residual — structured JSON tree inspector

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| Structured cells use multi-line `tree:` panel (not one-line `text:`) | done |
| Glyph indent (`│` / `└─`) for nested keys/values | done |
| Depth collapse past 6 levels (`{…` / `[…`) | done |
| Pretty output cap 16 KiB; line cap 64 | done |
| Invalid non-JSON structured text still falls back | done |

## Decision

No separate interactive tree widget yet: inspector projects a bounded
text tree from best-effort JSON pretty-print. Calendar widget remains
optional polish. Interactive expand/collapse is deferred.

## Evidence

```text
cargo test -p tablerock-tui --lib inspector
```

## Remaining work

- Interactive expand/collapse (optional)
- Calendar control for temporal staging (optional)
