# Phase 2 three-engine overlap evidence

Date: 2026-07-17

One Testcontainers contract test now starts pinned PostgreSQL 18.4, ClickHouse
26.3, and Redis 8.8 containers concurrently. It connects one real adapter
session per engine, then submits all three operations to one bounded
`EngineService` before consuming any operation event. The core observes three
queued operations simultaneously. Each operation preserves its engine page
identity, returns its expected three-row data set, and reaches only the observed
`Completed` outcome.

The fixture uses a two-event channel per operation. Draining PostgreSQL first
therefore allows the independently running ClickHouse and Redis producers to
meet their own bounded backpressure instead of relying on one unbounded shared
queue. Every event receive has a 30-second failure deadline; first-page and
whole-operation elapsed time are also required to remain below that ceiling.

A warm local Docker run on 2026-07-17 observed:

| Engine | First page after all submissions | Completed after all submissions |
|---|---:|---:|
| PostgreSQL | 1.998 ms | 2.418 ms |
| ClickHouse | 2.863 ms | 3.059 ms |
| Redis | 3.064 ms | 3.137 ms |

These overlap measurements remain diagnostic evidence from one host. The first
separate current-line pass/fail budgets are now recorded in
[`133-phase-2-current-line-performance-budgets.md`](133-phase-2-current-line-performance-budgets.md).
Repeated older-line, cancellation, allocation-count, and release-profile
measurements remain required.

Sources are TableRock-owned requirements, official client behavior, and direct
pinned-server tests. No external-product source or protected expression
influenced this checkpoint.
