# Phase 2 Redis Binary SCAN Foundation Evidence

## Checkpoint

The engine now contains its first Redis adapter boundary using exact `redis-rs`
1.4.0. It creates standalone multiplexed async connections with explicit RESP2
or RESP3 negotiation, selects one logical database during connection setup,
reads binary values, and converts binary SCAN keys into bounded immutable core
pages. All driver clients, connections, commands, and response values remain
private.

This is not the complete Redis spike. HSCAN, SSCAN, and ZSCAN were subsequently
closed by [research 141](141-phase-2-redis-collection-scans.md). TLS fixtures,
authentication, reviewed TTL mutations, Pub/Sub, timeout, and reconnect remain.
Key-level TTL read truth is proved in
[`139-phase-2-redis-ttl-truth.md`](139-phase-2-redis-ttl-truth.md). Pipeline partial
failures are proved in
[`138-phase-2-redis-pipeline-partial-failure.md`](138-phase-2-redis-pipeline-partial-failure.md),
and blocking-command isolation/cancellation are proved in
[`131-phase-2-redis-service-cancellation.md`](131-phase-2-redis-service-cancellation.md).

## Dependency decision

`redis` 1.4.0 is pinned with default features disabled and only
`tokio-rustls-comp`. This supplies async Tokio I/O plus rustls using native
certificate roots. It excludes ACL convenience APIs, scripts, streams,
geospatial helpers, BigInt, clusters, sentinels, connection manager, cloud
identity, native TLS, insecure TLS, and web-PKI root bundles from this
checkpoint. The crate declares Rust 1.88 MSRV and BSD-3-Clause, compatible with
the workspace Rust 1.95 baseline and license policy.

`redis` 1.4.0 unconditionally depends on `xxhash-rust` 0.8.17 for its public
value-digest support, even with defaults disabled. The first dependency gate
rejected that crate because BSL-1.0 was absent from the allowlist. BSL-1.0 is an
OSI-approved, FSF-free permissive license with notice obligations compatible
with TableRock distribution. Removing the edge would require forking the
selected official client; downgrading would violate the fixed dependency
decision. BSL-1.0 is therefore explicitly added to the workspace license
allowlist, and the accepted graph passes the license gate.

Context7 was attempted first and reported its monthly quota exhausted. Version,
features, MSRV, API behavior, and response shapes were verified from Cargo
metadata and the downloaded official `redis-rs` 1.4.0 source/documentation.

## Bounds, safety, and response truth

- Public operations are read-only `SCAN`, `GET`, and `CLIENT INFO`; no arbitrary
  Redis command or write bypass is exposed. Test seeding uses the driver only
  inside disposable integration fixtures.
- Connection Debug exposes host byte length, port, database, protocol, and TLS
  mode, never host text or response data.
- `RedisKeyStream` owns cursor, pending binary keys, requested scan count, and a
  finite total round budget. Empty SCAN batches continue until data, cursor zero,
  or budget exhaustion. A cursor is never discarded when a page fills.
- SCAN `COUNT` is a server hint, not a hard response cap. The driver necessarily
  receives one complete SCAN batch before TableRock can bound it; pending keys
  are then retained and pages obey row, arena, metadata, and per-cell limits.
- Binary key/value truncation preserves original byte length. Redis bytes are
  never interpreted as UTF-8.
- RESP negotiation is verified from `CLIENT INFO`. RESP2 bulk and RESP3
  verbatim/simple response shapes normalize internally to one closed protocol
  fact; the driver response enum cannot escape.
- Errors expose only closed message-free categories. SCAN budget exhaustion is
  distinct from transport/command failure.

## Testcontainers support matrix

| Server | Immutable official image | Evidence | Claim |
|---|---|---|---|
| Redis 7.4.9 | `redis:7.4.9-alpine@sha256:6ab0b6e7381779332f97b8ca76193e45b0756f38d4c0dcda72dbb3c32061ab99` | RESP2 and RESP3 negotiation; binary GET; bounded binary SCAN; cursor paging; database 0/1 isolation | binary scan tracer |
| Redis 8.8.0 | `redis:8.8.0-alpine@sha256:9d317178eceac8454a2284a9e6df2466b93c745529947f0cd42a0fa9609d7005` | same contract suite | binary scan tracer |

Redis 8.8.0 is the current GA line and 7.4.9 is a supported previous line as of
2026-07-16. The container images are test inputs and are not distributed with
TableRock. Testcontainers Rust 0.27.3 owns lifecycle and ephemeral mapped ports.

## Verification record

- Redis adapter unit tests: pass.
- Redis 7.4.9/8.8.0 × RESP2/RESP3 Testcontainers matrix: pass.
- Full workspace, lint, documentation, dependency, secret, English, and drift
  gates are recorded in the publishing commit.

External concepts: RESP2/RESP3 negotiation, binary-safe strings, cursor SCAN, logical databases
Public sources: <https://docs.rs/redis/1.4.0>, <https://redis.io/docs/latest/commands/scan/>, <https://redis.io/docs/latest/develop/reference/protocol-spec/>, <https://redis.io/docs/latest/operate/oss_and_stack/install/version-mgmt/>, <https://hub.docker.com/_/redis>
Implementation source: TableRock-owned adapter, core page contracts, and independent Testcontainers fixtures
Copied code/assets/text: none
