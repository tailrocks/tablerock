# PostgreSQL activity fixture container host

Date: 2026-07-22

## Failure and correction

Velnor run 29854847335 passed all seven ClickHouse real tests after evidence
598, then failed connecting the first PostgreSQL activity fixture. That fixture
used Testcontainers' mapped port with hard-coded loopback.

The activity test now obtains the PostgreSQL host and port from the same
container and reuses that endpoint for admin, sleeper, and restricted-role
sessions. Its permission-denied assertions remain unchanged.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p tablerock-engine --test postgres_real activity_signal_permission_denied_for_restricted_role -- --nocapture --test-threads=1`
  passed: 1 passed, 21 filtered out.
- Velnor hosted rerun required after push.

## Provenance

No external product reference influenced this fixture correction. Endpoint
selection follows Testcontainers 0.27.3's locked `get_host()` and
`get_host_port_ipv4()` contract.
