# Plan 012 residual — GoToLastIdentityColumn

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `go_to_last_identity_column` | done |
| Uses reverse identity-list order | done |
| Already-there no-op | done |
| Action GoPkLast | done |
| Unit test | done |

## Decision

GoPk jumps to the first identity key. Composite keys also need the last
listed identity column without typing. GoPkLast walks identity list reverse.

## Evidence

```text
cargo test -p tablerock-tui --lib go_to_first_identity
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for last-identity jump
