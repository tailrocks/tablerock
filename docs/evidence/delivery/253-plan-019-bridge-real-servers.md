# Plan 019 checkpoint — bridge real-server open/probe/fetch

Date: 2026-07-18

## What landed

- Engine-correct probe dispatch on the facade:
  - PostgreSQL: `PostgreSqlProbe::BoundedSeries` / statement path
  - ClickHouse: `ClickHouseProbe::TypedValues` (LZ4 connect)
  - Redis: `RedisKeyScan`
- Integration suite `crates/tablerock-ffi/tests/bridge_real.rs`:
  - `bridge_postgres_open_probe_fetch_shutdown`
  - `bridge_clickhouse_open_probe_fetch`
  - `bridge_redis_open_probe_fetch`
  - `bridge_three_engines_sequential_open_probe`
- Path under test: **UniFFI facade** `open` → `submit(probe)` → `pump` →
  `next_events` → `fetch_page` → `shutdown` against Docker fixtures (same image
  pins as engine real suites).
- Nested-runtime trap avoided: containers start on `#[tokio::test]`; bridge
  runs in `spawn_blocking` because the facade owns its multi-thread runtime.

## Verification

```bash
cargo test -p tablerock-ffi --test facade --test conformance
cargo test -p tablerock-ffi --test bridge_real
```

Observed: 13 unit/conformance passed; 4/4 real-server bridge tests passed
(~2.7s with warm local images).

CI: `checks.yml` real-servers job runs `cargo test -p tablerock-ffi --test bridge_real`.

## Residual (still operator-blocked)

- XCFramework via full Xcode
- Developer ID notarization / stapling / clean-machine Gatekeeper
