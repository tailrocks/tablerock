# Dependency Decisions

Versions and license metadata were checked on 2026-07-17. Every adoption starts
from the latest stable release, and every implementation checkpoint re-checks
the registry, upstream release, MSRV, features, advisories, duplicate graph,
maintenance, and license. Upgrade an outdated direct dependency immediately.
Exact pins make builds reproducible; they never authorize retaining a legacy
release. Any temporary upstream constraint requires evidence, a narrow audit
exception when necessary, and re-checking by daily automation. The selected
library does not change without a recorded architecture revision.

## Selected baseline

| Concern | Selected project | Researched version | License | Ownership rule |
|---|---|---:|---|---|
| Async runtime | [`tokio`](https://github.com/tokio-rs/tokio) | 1.52.4 | MIT | engine/runtime and CLI effect/subscription adapter only |
| Async stream adapter | [`futures-util`](https://github.com/rust-lang/futures-rs) | 0.3.32 | MIT OR Apache-2.0 | CLI EventStream polling only |
| PostgreSQL | [`tokio-postgres`](https://github.com/rust-postgres/rust-postgres) | 0.7.18 | MIT OR Apache-2.0 | driver adapter only |
| PostgreSQL TLS | [`tokio-postgres-rustls`](https://github.com/jbg/tokio-postgres-rustls) | 0.14.0 | MIT | PostgreSQL adapter only |
| TLS configuration | [`rustls`](https://github.com/rustls/rustls) | 0.23.42 | Apache-2.0 OR ISC OR MIT | PostgreSQL transport plus Redis root/identity validation; ring/std/TLS 1.2 only |
| Test certificates | [`rcgen`](https://github.com/rustls/rcgen) | 0.14.8 | MIT OR Apache-2.0 | engine development dependency only; ephemeral TLS fixtures |
| Real-server fixtures | [`testcontainers`](https://github.com/testcontainers/testcontainers-rs) | 0.27.3 | MIT OR Apache-2.0 | development dependency in driver test crates only |
| ClickHouse | official [`clickhouse`](https://github.com/ClickHouse/clickhouse-rs) | 0.15.1 | MIT OR Apache-2.0 | driver adapter only |
| Redis | [`redis`](https://github.com/redis-rs/redis-rs) | 1.4.0 | BSD-3-Clause | driver adapter only |
| SSH tunneling | [`russh`](https://github.com/Eugeny/russh) | 0.62.2 | Apache-2.0 | transport adapter below drivers |
| SQL analysis | [`sqlparser`](https://github.com/apache/datafusion-sqlparser-rs) | 0.62.0 | Apache-2.0 | Rust editor/query service |
| TUI | [`termrock`](https://github.com/tailrocks/termrock) | 0.11.0 / exact Git revision | Apache-2.0 | only reusable TUI layer |
| Terminal renderer | [Ratatui](https://github.com/ratatui/ratatui) | 0.30-compatible with pinned TermRock | MIT | through TermRock compatibility tuple |
| Terminal backend/input | [`crossterm`](https://github.com/crossterm-rs/crossterm) | 0.29.0 | MIT | CLI terminal adapter; TermRock `crossterm` feature |
| Ratatui terminal backend | [`ratatui-crossterm`](https://github.com/ratatui/ratatui) | 0.1.2 | MIT | CLI render adapter only; Crossterm 0.29 feature only |
| PTY test harness | [`portable-pty`](https://github.com/wezterm/wezterm) | 0.9.0 | MIT | CLI development dependency only |
| Secret zeroization | [`zeroize`](https://github.com/RustCrypto/utils/tree/master/zeroize) | 1.9.0 | MIT OR Apache-2.0 | core dangerous-local secret buffer only; `alloc` feature |
| Backup digest | [`sha2`](https://github.com/RustCrypto/hashes/tree/master/sha2) | 0.11.0 | MIT OR Apache-2.0 | persistence offline backup verification only; default features disabled |
| Persistence | [`turso`](https://github.com/tursodatabase/turso) local database | 0.7.0 | MIT | one serialized Rust async persistence actor |
| Unicode normalization | [`unicode-normalization`](https://github.com/unicode-rs/unicode-normalization) | 0.1.25 | MIT OR Apache-2.0 | core profile-search normalization only |
| Unicode case folding | [`caseless`](https://github.com/unicode-rs/rust-caseless) | 0.2.2 | MIT | core profile-search normalization only |
| Swift binding | [`uniffi`](https://github.com/mozilla/uniffi-rs) | 0.32.0 | MPL-2.0 | synchronous coarse FFI only |
| Structured diagnostics | [`tracing`](https://github.com/tokio-rs/tracing) | 0.1.44 | MIT | fixed safe fields only |
| Telemetry export | [`opentelemetry-otlp`](https://github.com/open-telemetry/opentelemetry-rust) | 0.32.0 | Apache-2.0 | opt-in OTLP, disabled by default |

TableRock does not directly depend on a separate textarea/widget framework,
`rusqlite`, `libsql`, Turso Cloud sync, Arrow, an Objective-C bridge from Rust,
a local RPC framework, or a second SQL parser. A missing general TUI primitive
is implemented in TermRock.

TableRock enables all features published by the exact TermRock pin:
`crossterm` and `serde`. The workspace owns that single feature declaration;
see
[`93-termrock-all-features-neutral-input-adoption.md`](93-termrock-all-features-neutral-input-adoption.md).

### Secret zeroization

The core pins `zeroize` 1.9.0 with default features disabled and only `alloc`.
It has no dependencies, supports the workspace Rust version, and gives `Vec`
compiler-resistant zeroing across its full capacity. TableRock uses it only for
the explicitly acknowledged dangerous-local plaintext owner, keeps that owner
non-cloneable, and retains the workspace-wide `unsafe_code = "forbid"` rule.
Reference-only secret variants never contain resolved bytes.

### Backup verification

The persistence crate pins latest stable `sha2` 0.11.0 with default features
disabled. It incrementally hashes bounded 64 KiB copy buffers into SHA-256 for
the versioned offline backup manifest; no digest or path crosses into profile
state. Cargo registry and official RustCrypto metadata report Rust 1.85 minimum
and MIT OR Apache-2.0. Context7 was attempted first and reported its monthly
quota exhausted. See
[`135-phase-2-persistence-backup-restore.md`](135-phase-2-persistence-backup-restore.md).

### Profile search normalization

The core pins `unicode-normalization` 0.1.25 and `caseless` 0.2.2. The tuple
implements NFKC and full default non-Turkic case folding without locale or OS
services. Normalization tables are Unicode 17.0 and fold tables are Unicode
16.0; that exact tuple defines search-normalization version 1. Context7 was
attempted first and reported its monthly quota exhausted, so current versions,
APIs, Unicode table constants, licenses, repositories, and dependency metadata
were verified from Cargo/crates.io and the downloaded official unicode-rs
sources. The only new transitive crates are `tinyvec` 1.12.0 and
`tinyvec_macros` 0.1.1; their declared licenses are Zlib/MIT/Apache-2.0 choices.
Neither normalization dependency crosses the core boundary.

### Phase 1 runtime adoption

The executable shell pins Tokio 1.52.4 with only macros, current-thread runtime,
signal, and sync features. `futures-util` exists only to poll Crossterm's one
`EventStream`; `ratatui-crossterm` is the renderer adapter matching the pinned
Ratatui/Crossterm tuple. `portable-pty` is test-only and runs the built CLI in a
real sized pseudoterminal. Cargo metadata and official crate documentation were
checked at adoption; Context7's documentation endpoint was attempted first but
reported its monthly request quota exhausted.

## PostgreSQL

`tokio-postgres` provides explicit async connection ownership, `query_raw`
streaming, prepared metadata, and `CancelToken`. The adoption checkpoint proves
arbitrary OIDs/extension types, rustls negotiation, cancellation races, COPY,
notices, multiple statements, portal/backpressure behavior, large values,
connection loss, and ambiguous writes. All client types terminate inside the
adapter.

The first adoption checkpoint pins `tokio-postgres` 0.7.18 and
`tokio-postgres-rustls` 0.14.0. The TLS adapter enables `ring` plus
`native-certs`; the rejected `webpki-roots` candidate was removed because its
CDLA-Permissive-2.0 data license is outside the approved license policy. See
[`81-phase-2-postgresql-stream-foundation.md`](81-phase-2-postgresql-stream-foundation.md).
The follow-up checkpoint removes the text-only feasibility path and proves
extended-query binary streaming on PostgreSQL 17.10 and 18.4; see
[`87-phase-2-postgresql-typed-stream.md`](87-phase-2-postgresql-typed-stream.md).
Real-server lifecycle uses exact Testcontainers Rust 0.27.3 with default
features disabled. It is a development dependency only; production engine
artifacts cannot start or control containers.

Use rustls with explicit platform/project root handling and client identity.
Invalid-certificate acceptance exists only as a dangerous, visible profile
setting and is never enabled by default.

The TLS feasibility matrix now uses direct rustls 0.23.42 configuration with a
deterministic ring provider, bounded custom CA/client identity PEM parsing, and
an independent server-name wrapper. Development-only rcgen 0.14.8 generates
ephemeral certificates; no key is committed. `Prefer` was deleted because the
core has no plaintext-downgrade policy. See
[`136-phase-2-postgresql-tls-identity.md`](136-phase-2-postgresql-tls-identity.md).

## ClickHouse

Use official `ClickHouse/clickhouse-rs`. Typed Row APIs handle known catalog
queries. Arbitrary workbench results use `fetch_bytes()` with one selected
self-describing ClickHouse format and convert immediately into owned TableRock
pages. Contract tests cover late errors, query IDs, progress, server
cancellation, nested/nullable/decimal/large integer/binary types, mutations,
compression, and TLS roots.

No hand-written ClickHouse HTTP client exists in the selected architecture.

The first adoption checkpoint pins `clickhouse` 0.15.1 with LZ4 and rustls
native-root support. It consumes the official client's cancellation-safe raw
chunk cursor, parses `RowBinaryWithNamesAndTypes` metadata, and proves bounded
typed pages against immutable ClickHouse 25.8 and 26.3 LTS Testcontainers
fixtures. See
[`95-phase-2-clickhouse-rowbinary-foundation.md`](95-phase-2-clickhouse-rowbinary-foundation.md).
The same single decoder now covers the complex scalar matrix through 256-bit
integers and decimals without precision loss; see
[`97-phase-2-clickhouse-complex-scalars.md`](97-phase-2-clickhouse-complex-scalars.md).
It also parses recursive `Array`, `Tuple`, `Map`, and named `Nested` type
signatures and emits bounded canonical structured projections; see
[`100-phase-2-clickhouse-structured-containers.md`](100-phase-2-clickhouse-structured-containers.md).

PostgreSQL JSON/JSONB projection pins `serde_json` 1.0.150 with `std` and
`arbitrary_precision`. A bounded counting writer avoids a full projected output
allocation, while an 8 MiB input ceiling bounds DOM allocation. See
[`168-phase-2-postgresql-json-projection.md`](168-phase-2-postgresql-json-projection.md).

## Redis

Use `redis-rs/redis-rs` with the minimum async, TLS, and protocol features needed
for the supported standalone deployment. Multiplexed connections handle normal
commands; dedicated connections isolate Pub/Sub and blocking commands. Contract
tests must prove byte safety, all SCAN variants, logical database isolation,
post-dispatch cancellation truth, command metadata, TLS, timeout, and reconnect.

The first adoption checkpoint pinned `redis` 1.4.0 with
`tokio-rustls-comp`; the bounded reconnect checkpoint adds only the official
`connection-manager` feature. Binary GET/SCAN, RESP2/RESP3, logical database
isolation, bounded connect/response handshakes, immediate setup cancellation,
and confirmed-drop reconnect pass immutable Redis 7.4.9 and 8.8.0
Testcontainers fixtures. `redis` declares Rust 1.88 and BSD-3-Clause. The newly
activated locked transitives are `backon` 1.6.0 (Rust 1.85, Apache-2.0),
`arc-swap` 1.9.2 (no declared `rust-version`, MIT OR Apache-2.0), and
`futures-channel` 0.3.32 (Rust 1.71, MIT OR Apache-2.0). Workspace Rust 1.97,
`cargo deny check`, and the full locked test/clippy/doc gates verify this graph.
The same rustls feature now passes generated custom-root and optional mTLS
identity fixtures plus ACL authentication without adding a dependency or an
insecure verifier. See
[`144-phase-2-redis-tls-authentication.md`](144-phase-2-redis-tls-authentication.md)
and
[`90-phase-2-redis-binary-scan-foundation.md`](90-phase-2-redis-binary-scan-foundation.md).
The official client unconditionally brings `xxhash-rust` 0.8.17 for value
digests. Its permissive OSI-approved BSL-1.0 license is explicitly accepted;
the dependency cannot be feature-disabled without forking the selected client.

## SQL analysis and editor

Use `sqlparser` tokens, dialect-aware delimiters, last-known-valid AST, catalog
index, bounded cursor context, and revisioned completion. Incomplete input falls
back to tokens; no second parser is introduced.

TermRock owns `TextArea` and `CompletionMenu`. TableRock supplies the editor
model, SQL/Redis spans, diagnostics, candidate ranking, statement semantics, and
effects. TableRock does not import a separate textarea widget.

## SSH transport

Use `russh` for client connections and direct-tcpip channels. One Rust tunnel
adapter below all three drivers owns host-key verification/known-hosts, key/
agent/password authentication, local forwarding, keepalive, cancellation,
reconnect, and redacted errors. The official client API returns a handle that
opens tunneling channels
([russh client](https://docs.rs/russh/latest/russh/client/)).

No shell command constructs an SSH tunnel. Database drivers receive the
established local stream/endpoint and remain unaware of SSH credentials.

## Ratatui and TermRock

TableRock uses Ratatui's **The Elm Architecture** application pattern as fixed
in [07-application-pattern.md](07-application-pattern.md). TermRock supplies the
terminal session, theme, input/focus/interaction contracts, reusable widgets,
runtime result/subscription types, lookbook, and conformance. TableRock supplies
the root Model/Message/Update/Effect/View and database compositions.

Render only resident rows and columns. Hot widgets use direct `Buffer` tests and
benchmarks; database fetch never occurs during rendering.

## Crossterm

Use Crossterm as the only terminal backend and input source. `tablerock-cli`
enables TermRock's `crossterm` feature and pins the same Crossterm 0.29 line as
the TermRock/Ratatui compatibility tuple. The CLI enables `event-stream` for one
async stream of keyboard, mouse, resize, focus, and paste events; the terminal
adapter maps them immediately into backend-neutral TermRock input and root TEA
messages.

TermRock's scoped Crossterm session is the only owner of raw mode, alternate
screen, mouse capture, bracketed paste, cursor restoration, and terminal
commands. Product widgets and reducers never call Crossterm, write escape
sequences, or restore the terminal independently. PTY tests cover partial setup,
normal exit, error, signal, panic, resize, paste, and double-restore prevention.

Official Crossterm 0.29 documentation covers the cross-platform command API,
raw/alternate-screen terminal control, keyboard/mouse/resize events, and async
`event-stream`
([crate](https://docs.rs/crossterm/latest/crossterm/),
[events](https://docs.rs/crossterm/latest/crossterm/event/),
[terminal](https://docs.rs/crossterm/latest/crossterm/terminal/)).

## Persistence

Use `turso` 0.7.0 with default features disabled and
`Builder::new_local`. One dedicated current-thread Rust async runtime hosts the
single persistence actor and owns all database handles. Commands serialize
migrations, transactions, queries, retention, flush, and shutdown; TUI, engine,
and Swift never access the file directly. Do not enable cloud sync/remote access
or add `rusqlite`/`libsql`
([Rust quickstart](https://docs.turso.tech/sdk/rust/quickstart)).

The pre-1.0 adoption spike must prove every required SQL/schema operation,
foreign-key behavior, crash boundaries, corruption detection, backup/restore,
macOS packaging, and terminal deployment against the pinned release. Avoid
experimental MVCC, FTS, encryption, multi-process access, and unsupported SQL;
use ordinary transactions plus indexed bounded history. Track upstream gaps
against [COMPAT.md](https://github.com/tursodatabase/turso/blob/main/COMPAT.md).
Failure blocks the checkpoint and never activates a fallback persistence crate.

The initial adoption pins 0.7.0 with default features disabled and proves
`Builder::new_local`, one bounded serialized owner, current-thread runtime,
transactional sequential migrations, foreign-key enforcement, rollback,
integrity, checkpoint, copy/reopen, and corruption rejection. Evidence and
remaining storage gates are recorded in
[58-phase-2-persistence-actor-foundation.md](58-phase-2-persistence-actor-foundation.md).

Persist profiles, secret references, organization, saved queries, preferences,
intent-only restoration, bounded history, and support metadata. Do not persist
resolved secrets, result pages, pending mutations, or automatic retry intent.

## UniFFI

Use a Rust `staticlib` plus UniFFI-generated Swift/header/module-map artifacts in
an XCFramework. Export synchronous coarse calls only: open, submit, bounded event
poll, fetch encoded page, cancel, and shutdown. Rust owns Tokio; a non-main Swift
actor invokes the facade and publishes immutable snapshots to `MainActor`.

The adoption gate requires Swift 6 strict concurrency, explicit handle
destruction, panic containment, operation-ID cancellation, bounded buffers,
deterministic generated artifacts, universal packaging/signing, leak checks,
and measured page/scroll performance.

## Diagnostics and telemetry

Use `tracing` spans/events with compile-time-reviewed safe fields. The executable
installs the subscriber; libraries do not set a global subscriber. Optional
OpenTelemetry uses `opentelemetry-otlp` HTTP/protobuf export with bounded batch,
timeout, and no exporter retry of database operations. Export is disabled by
default
([tracing](https://docs.rs/tracing/latest/tracing/),
[OTLP exporter](https://docs.rs/opentelemetry-otlp/latest/opentelemetry_otlp/)).

## Adoption checklist

1. Record source, exact version/revision, enabled features, license, MSRV, and
   provenance.
2. Run format, lint, test, docs, dependency, license, and advisory gates.
3. Review duplicate and unused dependencies.
4. Add an adapter; never expose dependency types through `tablerock-core`.
5. Add the contract test proving the motivating requirement.
6. Record cancellation, timeout, TLS, redaction, and failure semantics.
7. Update the fixed decision before substituting a selected dependency.
8. Re-run the latest-release check immediately before committing; never retain
   an older version merely for backward compatibility.
