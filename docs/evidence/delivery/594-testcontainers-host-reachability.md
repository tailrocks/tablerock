# Testcontainers host reachability

Date: 2026-07-22

## Failure class

Velnor CI runs 29849426803 and 29851685110 failed the first ClickHouse
readiness probe while the same pinned test passed locally. GitHub-hosted run
29852125099 then passed the complete initial 43-test real-server step at the
same commit. This isolates the failure to runner topology rather than server
startup time or test concurrency.

The ClickHouse fixtures asked Testcontainers for Docker's mapped port but
discarded its matching host and connected to hard-coded `127.0.0.1`.
Testcontainers 0.27.3 explicitly documents `ContainerAsync::get_host()` as the
host on which the container is reachable, which may differ from the local
machine.

## Correction

Every container-backed ClickHouse connection now obtains both host and mapped
port from the same running container. Shared readiness and cancellation helpers
receive that host explicitly. Loopback-only synthetic tests remain unchanged.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p tablerock-engine --test clickhouse_real explain_raw_and_structured_with_fallback --no-run`
- `cargo test -p tablerock-engine --test clickhouse_real explain_raw_and_structured_with_fallback -- --nocapture --test-threads=1`
  passed: 1 passed, 6 filtered out.
- Velnor hosted rerun required after push.

## Provenance

No external product reference influenced this fix. Evidence comes from
TableRock hosted runs and Testcontainers 0.27.3 source documentation installed
from the repository's exact lockfile resolution.
