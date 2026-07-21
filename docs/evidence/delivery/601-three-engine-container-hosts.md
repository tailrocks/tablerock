# Three-engine overlap container hosts

Date: 2026-07-22

## Correction

The explicit PostgreSQL/ClickHouse/Redis overlap test now pairs every mapped
port with the host reported by its own Testcontainers instance. PostgreSQL and
Redis sessions, the ClickHouse readiness helper, and Redis seeding all consume
those endpoints. Simultaneous three-engine scheduling semantics are unchanged.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p tablerock-engine --test three_engine_overlap_real -- --nocapture --test-threads=1`
  passed: 1 passed.
- Velnor hosted rerun required after push.

## Provenance

No external product reference influenced this fixture correction. Endpoint
selection follows Testcontainers 0.27.3's locked host/port contract.
