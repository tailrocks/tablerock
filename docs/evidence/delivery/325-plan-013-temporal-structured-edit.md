# Plan 013 residual — temporal / structured cell staging validation

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| Temporal staged text: ISO date/time/datetime (+Z/offset) | done |
| Structured staged text: JSON object/array/scalar balance | done |
| Reject injection-ish temporal noise (`;`, controls) | done |
| Unit: accept good / reject bad samples | done |

## Decision

Full calendar/JSON tree widgets remain optional polish. Commit gate now
validates temporal and structured distinctions instead of accepting any
string. Server remains authority for type coercion; fail closed on
obviously malformed staging text.

## Evidence

```text
cargo test -p tablerock-tui --lib temporal_and_structured
cargo test -p tablerock-tui --lib boolean_toggle
```

## Remaining work

- Multi-column FK follow polish
- Optional dedicated temporal/JSON UI widgets (beyond validation)
