# Plan 012 residual — CopyResultToken

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyResultToken decimal when nonzero | done |
| Zero token no-op | done |
| Action CopyTok | done |
| Unit test | done |

## Decision

`result_token` seeds FetchPage for the active result. Debug/support needs
the bare token without parsing status. CopyTok emits decimal text.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_result_token
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for result-token copy
