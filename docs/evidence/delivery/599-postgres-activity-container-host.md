# PostgreSQL activity fixture container host

Date: 2026-07-22

## Failure and correction

Velnor run 29854847335 passed all seven ClickHouse real tests after evidence
598, then failed connecting the first PostgreSQL activity fixture. That fixture
used Testcontainers' mapped port with hard-coded loopback.

All PostgreSQL real-server fixtures now obtain the host from their own
container through one local test macro while retaining the mapped port from
that same container. This covers 27 connection configurations, including TLS,
cancellation, paging, writes, catalog, roles, DDL, and activity. TLS fixtures
still use their independent certificate server-name override. Activity
permission-denied assertions remain unchanged.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p tablerock-engine --test postgres_real activity_signal_permission_denied_for_restricted_role -- --nocapture --test-threads=1`
  passed: 1 passed, 21 filtered out.
- PostgreSQL real-test binary compiled after all 27 endpoint replacements.
- Authorized transactional update/conflict real test passed.
- Velnor hosted rerun required after push.

## Provenance

No external product reference influenced this fixture correction. Endpoint
selection follows Testcontainers 0.27.3's locked `get_host()` and
`get_host_port_ipv4()` contract.
