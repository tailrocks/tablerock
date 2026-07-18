# Plan 013 residual — format/compact structured cell edit

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `format_structured` pretty-indent for Structured kind | done |
| `compact_structured` single-line collapse | done |
| Fail closed for non-JSON-like / wrong kind | done |
| Actions FmtJson / CmpJson | done |
| Unit test round-trip format → compact | done |

## Decision

Best-effort brace/bracket pretty printer without a JSON crate dependency.
Invalid text is left unchanged (fail closed). Interactive tree widget remains
optional.

## Evidence

```text
cargo test -p tablerock-tui --lib structured_format
cargo check -p tablerock-tui
```

## Remaining work

- Interactive expand/collapse tree editor (optional)
