# Plan 009 residual — Inspect toggle + CloseInspector

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `inspect_cursor` toggles closed when same title open | done |
| `close_inspector` force close | done |
| `ActionId::CloseInspector` | done |
| Toolbar CloseInsp | done |
| Unit test | done |

## Decision

Second Inspect on the same cursor cell closes the panel (toggle). Moving the
cursor and Inspecting again opens a new projection. Explicit CloseInspector
always clears.

## Evidence

```text
cargo test -p tablerock-tui --lib inspect_cursor_toggles
cargo check -p tablerock-tui
```

## Remaining work

- Focus-follows-inspector (optional)
