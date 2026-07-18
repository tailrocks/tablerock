# Plan 013 — PostgreSQL mutation apply seam (checkpoints 1–2 partial)

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `CommandIntent::ApplyMutations { review_token_id }` handle-only | done |
| Safety `MayWrite`, context scope | done |
| `PostgresSession::apply_authorized_mutation` | done |
| Single transaction; ≠1 rows → conflict + ROLLBACK | done |
| Wire-type casts (`$n::bigint`/`text`/…) so INT4 columns accept i64 binds | done |
| Real-server: update, zero-row conflict, insert+delete multi-change, PK violation, session health | done |
| RETURNING / generated-value reconciliation | deferred (next checkpoint) |
| Ambiguity inject (deferred-trigger) → `Unknown` | deferred (next checkpoint) |
| Staged-edit model / review UI | later checkpoints |

## Design notes

- SQL is built only from `quote_ident` + typed `$n::cast` parameters. Preview text is never executed.
- Parameter casts match the rustls/tokio-postgres wire type we bind. Without them, prepare infers column types (e.g. INT4) and rejects `i64` serialization.
- `ServerCancelled` maps to `MutationTransactionState::Unknown` (no replay).
- COMMIT failure after successful changes also maps to `Unknown`.

## Verification

```text
cargo test -p tablerock-core --test command
cargo test -p tablerock-engine --lib postgres_mutation
cargo test -p tablerock-engine --test postgres_real applies_authorized_update
```

## Provenance

Clean-room from `docs/product/editing.md`, core mutation typestate
(`crates/tablerock-core/src/mutation.rs`), Redis TTL apply shape
(`apply_reviewed_ttl_mutation`), and plan 012 quoting/parameter discipline.
No third-party product source.
