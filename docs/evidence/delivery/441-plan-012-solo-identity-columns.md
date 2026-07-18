# Plan 012 residual — SoloIdentityColumns

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `solo_identity_columns` | done |
| Empty identity no-op | done |
| Already-solo no-op | done |
| At least one visible fail-closed | done |
| Action ColPk | done |
| Unit test | done |

## Decision

ColSolo keeps one cursor column. Reviewing composite keys needs all
identity columns visible and others hidden. ColPk uses
`identity_columns` facts without changing query identity.

## Evidence

```text
cargo test -p tablerock-tui --lib solo_identity
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for identity column solo
