# Plan 018 residual — disconnect mid-stream marks live ops

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `GridOperationState::is_live` | done |
| Live grids flip to `Disconnected` on session loss | done |
| Completed/failed grids stay terminal (inspectable) | done |
| Late `GridPage` / stream complete / fail / cancel ignored | done |
| Unit: workbench + update mid-stream | done |

## Decision

Product truth: disconnect keeps stale results inspectable and marks every
**live** operation disconnected. `mark_disconnected` previously only cleared
tab `running` flags. It now sets live grid operations to `Disconnected` and
the update loop ignores late stream events so a finishing page cannot revive
a disconnected tab.

## Evidence

```text
cargo test -p tablerock-tui --lib disconnect
```

## Remaining work

- Engine-level disconnect-while-borrowed races already covered in
  `engine_service` / session registry; no product claim of server cancel on
  disconnect alone.
