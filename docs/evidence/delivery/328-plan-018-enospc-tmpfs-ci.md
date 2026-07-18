# Plan 018 residual — true ENOSPC on 1MiB tmpfs CI

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `enospc_volume_fails_closed_without_temp_debris` | done |
| Env gate `TABLEROCK_ENOSPC_MNT` (skip without volume) | done |
| Ubuntu CI: 1MiB tmpfs + prefill + test | done |
| No `tablerock-tmp` debris after ENOSPC | done |

## Decision

True ENOSPC is host-specific. CI creates a 1MiB tmpfs, prefills it, and runs
the AtomicFileWriter fail-closed test under `TABLEROCK_ENOSPC_MNT`. Without
the env var the test no-ops so developer machines are never filled.

## Evidence

```text
# local skip path
cargo test -p tablerock-cli --lib enospc_volume_fails_closed

# CI (ubuntu real-servers job)
# sudo mount -t tmpfs -o size=1M ...
# TABLEROCK_ENOSPC_MNT=... cargo test -p tablerock-cli --lib enospc_volume_fails_closed
```

Workflow: `.github/workflows/checks.yml` step `ENOSPC fail-closed on 1MiB tmpfs`.

## Remaining work

- Fixed-spec multi-runner first-paint numbers beyond ubuntu budgets (optional)
