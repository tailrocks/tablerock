# Plan 013 residual — mutation apply Unknown (ambiguity inject)

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `WriteOutcomeUnknown` → `MutationTransactionState::Unknown` | done |
| Deferred constraint trigger fixture (`mut_ambiguous`) | done |
| Stop server during deferred COMMIT sleep | done |
| Apply outcome is Unknown (not Committed/RolledBack) | done |
| Single attempt only (no automatic retry in fixture) | done |
| Docker real test green | done |

## Decision

Probe-level ambiguity (`ambiguous_*_probe`) already maps transport/timeout
loss to `WriteOutcomeUnknown`. The product mutation path now maps that plus
`ServerCancelled` and COMMIT failure to `MutationTransactionState::Unknown`
so UI/CLI never claims rollback or success when the terminal state is
unconfirmed. Deferred INITIALLY DEFERRED trigger holds COMMIT in `pg_sleep`
so the fixture can kill the server after the write is dispatched.

## Evidence

```text
cargo test -p tablerock-engine --test postgres_real mutation_apply_commit_loss_is_unknown_without_retry
```

## Remaining work

- Full ValueKind editor widgets beyond kind gates
- Multi-column FK follow polish; indexes/constraints raw DDL depth
