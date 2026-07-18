# Plan 014 residual — ClickHouse KILL MUTATION gate

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `ClickHouseSession::kill_mutation` bound params + charset fail-closed | done |
| `system.mutations` poll includes `is_killed` | done |
| `DriverSession::kill_clickhouse_mutation` trait surface | done |
| TUI `KillMutation` + re-type confirm dialog | done |
| CLI effect `KillClickHouseMutation` | done |
| Docker: ALTER → bound KILL MUTATION + hostile reject | done |
| Unit: TUI hostile/wrong id stays gated | done |

## Decision

ClickHouse async UPDATE/DELETE create server mutations that outlive the
client statement. Cancellation of those mutations is a separate destructive
control: `KILL MUTATION WHERE database/table/mutation_id` with named
parameters only. Empty or non-id tokens fail closed before any network call.
The TUI requires the operator to paste the exact `mutation_id` after the
action is selected; free SQL is never accepted.

## Bounds and failure truth

- Identifiers: database/table non-empty; mutation_id charset
  `[A-Za-z0-9._-]{1,128}` (matches server `mutation_N.txt` ids).
- Non-ClickHouse engines: `AdapterFailureClass::EngineMismatch`.
- TUI without base table or non-CH session: action is a no-op.
- Kill of already-finished mutations is best-effort server semantics
  (empty match is not a false "applied" claim).

## Evidence

```text
cargo test -p tablerock-engine --lib kill_mutation_id_charset
cargo test -p tablerock-tui kill_mutation_requires_retype
cargo test -p tablerock-engine --test clickhouse_real kill_mutation_accepts_bound_id
```

## Remaining work

- Progress/query-id OperationEvent status bar (plan 014 residual).
- Custom CA / mTLS HttpClient fixture matrix.
- Instruments leak pass / packaging remain plan 019 operator gates.
