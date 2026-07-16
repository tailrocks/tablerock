# Phase 2 current-line performance budgets

Date: 2026-07-17

## Decision

TableRock now has continuously executed, conservative Phase 2 streaming
guardrails for the current production server line of each engine. This closes
the prior diagnostic-only gap without presenting one workstation's timing as a
portable release benchmark.

The real Testcontainers harness starts PostgreSQL 18.4, ClickHouse 26.3 LTS,
and Redis 8.8 concurrently. It then measures each adapter sequentially through
the shared object-safe `DriverSession`/`DriverPageStream` contract over 10,000
rows with the fixed 500-row product page size.

| Fact | Enforced budget |
|---|---:|
| Query start through first immutable page | at most 5 seconds |
| Query start through terminal stream exhaustion | at most 15 seconds |
| Steady whole-stream throughput | at least 500 rows/second |
| Rows per immutable page | at most 500 |
| One page's owned resident buffer capacity | at most 2 MiB |
| Test-process resident set size | at most 512 MiB |

The query cap is exactly 10,000 rows. PostgreSQL and ClickHouse use dedicated
bounded series probes. Redis seeds 10,000 bounded keys and iterates cursor SCAN
with a 500-row hint; SCAN remains allowed to return duplicates, so the adapter
must deliver at least the complete seeded cardinality while every page remains
bounded. All query/value text remains disposable fixture data and never enters
diagnostics.

Process RSS is sampled through the platform `ps` command before and after the
three streams. Missing RSS evidence fails rather than silently skipping the
memory gate. Page memory uses `ResultPage::resident_buffer_bytes`, which counts
owned allocation capacity instead of encoded logical length.

## Measurement environment and method

- Apple M1 Max
- macOS 26.5.2 build 25F84
- Rust 1.97.0
- Docker Engine 29.4.0
- ordinary unoptimized integration-test profile
- one warm run after image availability; containers use ephemeral mapped ports
- exact observations are emitted in the test log while fixed ceilings determine
  pass/fail

This is an early correctness/performance regression gate. Phase 2 still needs
repeated older-line, cancellation-latency, allocation-count, reconnect, and TLS
measurements. Release phases still require optimized artifacts, cold/warm CLI
startup, TUI scrolling, Turso, UniFFI, and Instruments evidence. These initial
budgets do not claim those later gates.

## Evidence and provenance

- `cargo test -p tablerock-engine --test performance_real -- --nocapture`
- full workspace, strict lint, rustdoc, dependency, English, and drift gates
- official pinned database images and selected client APIs only
- no external-product performance claim, source, or protected expression used
