# Plan 013 residual — structure indexes + constraints

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `PostgresSession::relation_indexes` | done |
| `PostgresSession::relation_constraints` | done |
| Structure inspector sections: columns / indexes / constraints | done |
| Docker: secondary index + CHECK + PK/FK defs | done |

## Decision

Structure panel is presentation lines, not a separate schema model.
`ShowStructure` loads column rows then index `pg_get_indexdef` and
constraint `pg_get_constraintdef` sections so operators see DDL-shaped
facts without leaving the workbench. Bounded to 128 indexes/constraints.

## Evidence

```text
cargo test -p tablerock-engine --test postgres_real applies_authorized_update_in_transaction
cargo check -p tablerock-cli
```

## Remaining work

- Full ValueKind editor widgets (bool/temporal/JSON/bytes) beyond kind gates
- Multi-column FK follow polish
