# CLI real-server container hosts

## Finding

Velnor run 29863328739 proved the Redis replacement and UniFFI bridge fixes,
then exposed the same hidden topology assumption in the CLI CSV import suite:
the test used Docker's mapped port but always connected to `127.0.0.1`.
Nested Docker publishes mapped ports on the daemon host, so readiness exhausted
its 30-second bound with `clickhouse not ready: Query`.

The architecture permitted the class because CLI and performance fixtures
retrieved only `get_host_port_ipv4`; their connection construction discarded
testcontainers' authoritative `get_host()` value. These paths escaped the
earlier engine and bridge host-reachability correction because each suite
owned separate connection setup.

## Correction

Every remaining real-server suite executed by the integration workflow now
gets both host and mapped port from its container and carries that pair through
database sessions, Redis seeding, and external PostgreSQL client invocations:

- CSV import apply;
- streaming export;
- `pg_dump` / `pg_restore`;
- first-row performance budgets.

No retry, timeout increase, lane conditional, or fixture weakening is added.

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy -p tablerock-cli --test import_apply_real --test stream_export_real --test pg_dump_real -p tablerock-engine --test performance_real --locked -- -D warnings`
- `cargo nextest run -p tablerock-cli --test import_apply_real applies_csv_insert_rows_on_clickhouse_progressive --locked --run-ignored ignored-only` — 1 passed.
- `cargo nextest run -p tablerock-cli --test stream_export_real --test pg_dump_real --locked --run-ignored ignored-only` — 2 passed.
- `cargo nextest run -p tablerock-engine --test performance_real --locked` — 1 passed.
- Velnor run 29863328739: hostile real-server matrix and UniFFI bridge passed;
  CSV import then reproduced the uncovered host-assumption class.
- Exact-head Velnor and dual-lane proof remain required after delivery.
