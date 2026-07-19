# Native Redis sampled overview

Date: 2026-07-19

## Contract

The shared Redis adapter owns the INFO request and projects a bounded curated
snapshot: version/mode, uptime, current/peak memory, connected clients,
operations per second, hit/miss counters, RDB/AOF state, and up to sixteen
logical-database key summaries. Every required absent field becomes an
explicit `unavailable (INFO field absent)` value. One Rust-produced Unix-epoch
sample timestamp applies to the whole snapshot.

UniFFI accepts only an opaque live session ID, rejects non-Redis sessions, and
returns the typed timestamp plus bounded lines. Swift performs no INFO parsing
and renders loading, empty, error, refresh, and selectable-value states in a
native sheet.

## Evidence

- Redis 8.0 live native fixture opens the Overview sheet and observes version,
  memory, and logical-database facts produced by Rust.
- Runtime state proves the sample timestamp is nonzero.
- UniFFI conformance proves the opaque Redis session route and typed snapshot.
- The verifier asserts the shared adapter's curated and explicit-unavailable
  contract before launching the native fixture.
- Engine library suite: 110 passed, including direct missing-field and
  sixteen-database-bound coverage.

## Remaining boundary

Historical sampling is intentionally absent; this is current server state
only. PostgreSQL/ClickHouse administrative dashboards and action parity remain
separate ledger work.

## Provenance

TablePro established only the broad server-overview workflow. No source,
tests, text, screenshots, layouts, measurements, colors, assets, or key
bindings were copied or translated. Implementation follows this repository's
Redis requirements, redis-rs behavior, and direct Redis tests.
