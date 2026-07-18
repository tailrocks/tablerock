# Plan 007 residual — CopyEnvironment / CopySafetyLabel / CopyHealthLabel

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyEnvironment when set | done |
| CopySafetyLabel | done |
| CopyHealthLabel | done |
| Empty fail closed | done |
| Actions CopyEnv / CopySafe / CopyHlth | done |
| Unit test | done |

## Decision

Support tickets and environment audits need discrete context-bar fields without
parsing the composed line. CopyEnv fails closed when no tag is set; safety and
health copy their current labels.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_session_id_and_engine
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for env/safety/health copy
