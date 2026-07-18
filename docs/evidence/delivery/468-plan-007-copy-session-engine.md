# Plan 007 residual — CopySessionId / CopyEngineLabel

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopySessionId from live session | done |
| CopyEngineLabel from live session | done |
| No session no-op | done |
| Actions CopySid / CopyEng | done |
| Unit test | done |

## Decision

Debug tickets need session correlation and engine identity without parsing
the context bar. CopySid/CopyEng emit the live session fields only.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_session_id_and_engine
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for session/engine copy
