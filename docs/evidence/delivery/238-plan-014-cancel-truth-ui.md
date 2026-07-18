# Plan 014 — Four-state cancel presentation

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `GridCancelDispatched.dispatch` carries CancelDispatch fact | done |
| Distinct labels: cancel requested / client stopped / server confirmed / cancel unknown | done |
| Reducer test covers four states | done |

## Verification

```text
cargo test -p tablerock-tui --lib cancel_dispatch
cargo test -p tablerock-tui --lib
cargo test -p tablerock-cli --lib
```
