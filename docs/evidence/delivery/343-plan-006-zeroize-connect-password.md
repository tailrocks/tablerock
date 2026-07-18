# Plan 006 residual — zeroize resolved connect password

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `tablerock-cli` depends on workspace `zeroize` | done |
| `open_driver_session` holds password in `Zeroizing<String>` | done |
| Drop scrubs attempt-scoped material on all return paths | done |
| Ledger: cancel/terminate gates marked closed (327) | done |

## Decision

Resolved password lives only for the connect attempt. `Zeroizing` ensures
the heap buffer is overwritten when the binding drops, including early
error returns after resolution.

## Evidence

```text
cargo check -p tablerock-cli
```

## Remaining work

- Avoid cloning secret into draft.password for DangerousPlaintext when loading
  profiles for edit (separate residual)
