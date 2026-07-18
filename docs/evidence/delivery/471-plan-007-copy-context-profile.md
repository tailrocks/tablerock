# Plan 007 residual — CopyContextBar / CopyConnectionName / CopyProfileId

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyContextBar full line | done |
| CopyConnectionName | done |
| CopyProfileId when set | done |
| Empty fail closed | done |
| Actions CopyCtx / CopyConn / CopyProf | done |
| Unit test | done |

## Decision

Session tickets need connection/profile correlation without screenshotting
the context bar. CopyCtx dumps the composed line; CopyConn/CopyProf emit
the discrete fields.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_session_id_and_engine
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for context/profile copy
