# Redis fixture container-host registry

Date: 2026-07-22

## Failure and correction

Velnor run 29855308857 passed all seven ClickHouse and all twenty-two
PostgreSQL real tests, then failed the first Redis fixture because it combined
a mapped Docker port with loopback.

Redis real tests now register each container's authoritative host by mapped
port. Adapter configs, raw redis-rs connections, inspectors, seeders, and
direct Redis URLs resolve through that bounded process-local registry. Ports
for synthetic local TCP/TLS fixtures are absent from the registry and retain
loopback, keeping their isolation semantics unchanged.

## Verification

- Redis real-test binary compiled after endpoint replacement.
- Multi-type collection mutation real test passed.
- Full supported-version/protocol binary-scan matrix passed.
- `cargo fmt --all -- --check`
- Velnor hosted rerun required after push.

## Provenance

No external product reference influenced this test infrastructure. Endpoint
selection follows Testcontainers 0.27.3's locked `get_host()` and mapped-port
contract.
