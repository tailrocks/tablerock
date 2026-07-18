# Plan 013 — RETURNING generated-value reconciliation

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| INSERT/UPDATE/DELETE use `RETURNING *` | done |
| `MutationChangeOutcome::Applied.returned` carries (column, display) | done |
| Docker: update RETURNING name; insert IDENTITY id | done |
| UI grid re-key from returned values | open (rebrowse covers) |
| Ambiguity inject → Unknown | open |

## Verification

```text
cargo test -p tablerock-engine --lib postgres_mutation
cargo test -p tablerock-engine --test postgres_real applies_authorized_update
```
