# Phase 2 three-engine service harness

Date: 2026-07-17

One reusable real-test harness now constructs the same bounded core scope,
command budget, operation identity, page identity, `ServiceCoordinator`, and
`DriverRuntime` for PostgreSQL, ClickHouse, and Redis. Every engine submits its
real Testcontainers session through `EngineService` and consumes only
`Started`, immutable bounded pages, and an observed `Completed` terminal fact.

PostgreSQL proves three typed rows across two pages on PostgreSQL 18.4.
ClickHouse proves three self-describing rows across two pages for uncompressed
and LZ4 requests on both pinned server lines. Redis proves the complete binary
key set across bounded SCAN pages for RESP2 and RESP3 on both pinned server
lines. Concrete clients, streams, rows, HTTP chunks, and Redis connections stay
behind adapters.

The harness removes duplicated core/service setup from the PostgreSQL test and
prevents engine-specific lifecycle normalization. Engine differences remain in
typed `DriverPageRequest` variants and capabilities; lifecycle, paging,
progress, shutdown, identity, bounds, and redaction use one contract.

This is execution-contract evidence, not Phase 2 exit: overlapping-operation
measurements, remaining protocol/TLS/COPY/pipeline/cancellation spikes, and
published budgets remain required. Sources are TableRock-owned requirements,
official clients, and direct pinned-server tests. No external-product source or
protected expression influenced this checkpoint.
