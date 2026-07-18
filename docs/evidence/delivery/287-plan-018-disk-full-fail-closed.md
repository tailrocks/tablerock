# Plan 018 residual — disk-full class fail-closed injection

Date: 2026-07-18

## What landed

Portable stand-ins for disk-full / write-failure class (true ENOSPC is
host-specific; permission-denied and mid-stream failure cover the same
fail-closed policy):

### Export (`tablerock-cli` `file_effects`)
- `create_fails_closed_when_parent_is_a_file` (existing, evidence 280)
- **`create_fails_closed_on_readonly_parent`** (unix) — 0555 parent → create
  errs, no `tablerock-tmp` debris
- **`write_fails_closed_when_file_becomes_unwritable`** (unix) — mid-stream
  path; dest never partially promoted

### Persistence SQL files
- **`write_fails_closed_on_readonly_parent`** — atomic SQL write fails closed,
  no `.tmp.*` debris, destination absent

## Commands

```bash
cargo test -p tablerock-cli --lib file_effects
cargo test -p tablerock-persistence write_fails_closed
```

## Residual

- True ENOSPC on a tiny volume image as scheduled CI (needs runner setup)
