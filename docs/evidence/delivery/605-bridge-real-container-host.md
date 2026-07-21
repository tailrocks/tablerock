# UniFFI bridge real-server container host

Date: 2026-07-22

## Failure class

Exact-main dual-lane run 29861711200 passed the Velnor format, lint, unit,
dependency, and engine real-server gates. The UniFFI bridge ClickHouse case
then exhausted its readiness retries because `bridge_real.rs` combined
Testcontainers' mapped port with hardcoded `127.0.0.1`. A remote Docker daemon
requires the authoritative host returned by the container handle.

## Correction

Every bridge real-server fixture now captures `container.get_host()` and carries
that host with its mapped port through readiness probes, Redis seeding, bridge
open parameters, mutation coverage, and the three-engine sequence. This removes
the remaining localhost assumption from the bridge suite without changing
production connection behavior.

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy -p tablerock-ffi --test bridge_real --locked -- -D warnings`
- `cargo nextest run -p tablerock-ffi --test bridge_real bridge_clickhouse_open_probe_fetch --locked --run-ignored ignored-only`
  passed locally.
- Exact-main GitHub/Velnor proof remains required after push.

## Provenance

The fix applies the existing Testcontainers endpoint contract documented in
evidence 594, 600, and 604 to the last uncovered real-server suite.
