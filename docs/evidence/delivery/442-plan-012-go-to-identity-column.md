# Plan 012 residual — GoToIdentityColumn

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `go_to_first_identity_column` | done |
| Uses identity list order | done |
| Already-there no-op | done |
| Reveals viewport | done |
| Action GoPk | done |
| Unit test | done |

## Decision

GoCol needs a name paste. After ColPk or while browsing wide rows, jump
to the first identity column without typing. GoPk uses identity facts
order (composite keys: first listed key).

## Evidence

```text
cargo test -p tablerock-tui --lib go_to_first_identity
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for go-to-identity column
