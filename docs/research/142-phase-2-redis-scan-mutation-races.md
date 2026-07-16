# Phase 2 Redis SCAN Mutation-Race Evidence

Date: 2026-07-17

## Decision

TableRock treats SCAN, HSCAN, SSCAN, and ZSCAN as live cursor iterations, never
as snapshots. A full iteration has only the official Redis guarantees:

- every element present continuously from iteration start through completion is
  returned at least once;
- an element absent before iteration start and throughout iteration is never
  returned;
- an element added, removed, or changed during iteration may be returned or
  omitted; and
- the same element may be returned multiple times.

The adapter therefore preserves unknown totals, forwards duplicate rows, and
does not deduplicate because doing so would add unbounded state. Presentation
and downstream consumers must tolerate repeated rows and must not derive a
stable completion percentage. A bounded scan-round budget prevents an
unbounded growing collection from holding the client forever; exhaustion is an
explicit query failure, not partial success.

Mutation is not performed by the scan API. The test mutator uses an independent
connection solely to create the server race while the TableRock session remains
read-only. Dropping a scan requires no server cancellation because Redis stores
no server-side iterator state.

## Evidence

Testcontainers Rust 0.27.3 runs immutable official Redis 7.4.9 and 8.8.0
images. Both RESP2 and RESP3 execute full TableRock iterations over:

- database keys through SCAN;
- hash fields through HSCAN;
- set members through SSCAN; and
- sorted-set members through ZSCAN.

Each fixture contains 601 stable identifiers, including a non-UTF-8 binary
identifier, plus a transient element and an element deleted before iteration.
Page size and COUNT are eight, while
the accepted batch cap is 128, forcing the first successful partial page to
belong to an unfinished server iteration rather than a single compact reply.
After that page, an independent connection removes the transient element and
adds a late element. Completion proves every stable identifier appeared and the
removed-before identifier did not. Results are accumulated in a set so legal
duplicates cannot make the test or product consumer fail. No assertion is made
for the transient or late identifier because Redis defines that outcome as
undefined.

This closes the Phase 2 concurrent SCAN-family mutation-race gate. The strict
pre-decode transport allocation cap, TLS/authentication, Pub/Sub,
timeout/reconnect, reviewed TTL mutation, complete type views, service/UI
integration, and native presentation remain open.

Context7 was attempted first and reported its monthly quota exhausted. The
redis-rs 1.4.0 command/reply path was verified from exact pinned source. Redis
primary documentation supplies the full-iteration, duplication, mutation,
cursor, COUNT-hint, server-state, and bounded-size termination rules.

## Provenance

External concept: Redis live cursor iteration under concurrent mutation  
Public sources: <https://redis.io/docs/latest/commands/scan/> and
<https://docs.rs/redis/1.4.0>  
TableRock requirements: research 03, 06, 10, 14, 20, 30, 31, 32, 90, and 141  
Implementation source: TableRock-owned Testcontainers fixtures over the public
bounded stream and adapter contracts  
Copied code/assets/text: none
