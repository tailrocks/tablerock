# Plan 007 residual — CopySessionIdentity / CopySessionStatus / CopyWorkbenchStatus

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopySessionIdentity when connected | done |
| CopySessionStatus when set | done |
| CopyWorkbenchStatus summary | done |
| Empty/disconnected fail closed | done |
| Actions CopyIdent / CopySessSt / CopyWbSt | done |
| Unit test | done |

## Decision

Support tickets need the live session identity string and status text, and the
workbench status summary, without screenshotting chrome. Discrete from grid
CopyStatus (active grid status line).

## Evidence

```text
cargo test -p tablerock-tui --lib copy_session_id_and_engine
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for session/workbench status copy
