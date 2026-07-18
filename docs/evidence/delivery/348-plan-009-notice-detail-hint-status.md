# Plan 009 residual — NOTICE detail/hint on status lines

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `drain_server_notices` includes optional detail and hint | done |
| Still redacted (no SQL/values); severity + message + detail + hint | done |
| Overflow line unchanged | done |

## Decision

Evidence 160 proved detail/hint on the notice type. Status projection now
appends them when present so operators see RAISE DETAIL/HINT without an
inspector history queue.

## Evidence

```text
cargo check -p tablerock-engine -p tablerock-cli
```

## Remaining work

- Inspector notice history — shipped as evidence 372
