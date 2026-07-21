# 574 — Retire unsafe support facts

Date: 2026-07-21

## Decision

Persistence schema 18 removes the dormant
`support_facts(fact_key TEXT, fact_value TEXT)` table. It had no production
reader or writer, and its arbitrary text values violated Phase 15's closed
support-diagnostic rule: accidental adoption could persist messages, SQL,
values, paths, endpoints, or credentials.

Migration 0018 uses `DROP TABLE IF EXISTS`. This is required because a legacy
backfill fixture proves databases can carry the historical migration ledger
without the unused table; upgrading such a database must not fail. Migration
0002 remains immutable history.

Future durable support retention requires a new bounded typed schema. The
current native runtime collector remains in memory and accepts only Rust enums.

## Verification

```text
cargo test -p tablerock-persistence --locked
cargo clippy -p tablerock-persistence --all-targets --locked -- -D warnings
```

Results: 39 tests across 12 suites pass; clippy reports no issues. The schema-18
test creates a schema-17 database with a credential-bearing arbitrary support
fact, upgrades it, and proves the table no longer exists. Fresh, interrupted,
legacy-group-backfill, crash-recovery, history, and profile migration paths all
reach schema 18.

## Remaining boundary

No support diagnostic is persisted. The TUI still has no long-lived collector,
and explicit bridge runtime destruction clears native retained outcomes.

## Provenance

Implementation source: TableRock-owned persistence and safe-support contracts.

TablePro influence: none; this is local-data security and migration work.

Copied source, tests, identifiers, assets, strings, colors, geometry, layout
measurements, or key bindings: none.
