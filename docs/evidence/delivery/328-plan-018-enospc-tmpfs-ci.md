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

The job builds a nextest archive, asks `mise which cargo-nextest` for the
active installation binary instead of copying its PATH shim, copies that binary and the
archive into the Ubuntu container, creates nextest's required remapped root
manifest and package directory, remaps the archived workspace metadata there,
and executes the exact test through nextest. This preserves the real tmpfs
boundary without a forbidden
`cargo test --no-run` build path or direct libtest invocation.

## Evidence

```text
# local skip path
cargo nextest run -p tablerock-files --lib \
  -E 'test(=tests::enospc_volume_fails_closed_without_temp_debris)'

# CI (ubuntu real-servers job)
# cargo nextest archive ...
# docker run --tmpfs /enospc:rw,size=1m ...
# /cargo-nextest nextest run --archive-file ... --workspace-remap /workspace \
#   -E 'test(=tests::enospc...)'
```

Workflow: `.github/workflows/ci.yml` step `ENOSPC fail-closed on 1MiB tmpfs`.

Hosted CI run `29871821771` passed this exact step on commit `2a6e06b` after
nextest archive extraction into the explicit `/workspace` remap.

## Remaining work

- Fixed-spec multi-runner first-paint numbers beyond ubuntu budgets (optional)
