# Dependency Decisions

Versions and license metadata were checked on 2026-07-16. Re-resolve the exact
version, MSRV, features, advisories, duplicate graph, maintenance, and license in
the approved checkpoint that adds each dependency. The selected library does not
change without a recorded architecture revision.

## Selected baseline

| Concern | Selected project | Researched version | License | Ownership rule |
|---|---|---:|---|---|
| Async runtime | [`tokio`](https://github.com/tokio-rs/tokio) | 1.52.3 | MIT | engine/runtime and CLI effect/subscription adapter only |
| Async stream adapter | [`futures-util`](https://github.com/rust-lang/futures-rs) | 0.3.32 | MIT OR Apache-2.0 | CLI EventStream polling only |
| PostgreSQL | [`tokio-postgres`](https://github.com/rust-postgres/rust-postgres) | 0.7.18 | MIT OR Apache-2.0 | driver adapter only |
| PostgreSQL TLS | [`tokio-postgres-rustls`](https://github.com/jbg/tokio-postgres-rustls) | 0.14.0 | MIT | PostgreSQL adapter only |
| ClickHouse | official [`clickhouse`](https://github.com/ClickHouse/clickhouse-rs) | 0.15.1 | MIT OR Apache-2.0 | driver adapter only |
| Redis | [`redis`](https://github.com/redis-rs/redis-rs) | 1.4.0 | BSD-3-Clause | driver adapter only |
| SSH tunneling | [`russh`](https://github.com/Eugeny/russh) | 0.62.2 | Apache-2.0 | transport adapter below drivers |
| SQL analysis | [`sqlparser`](https://github.com/apache/datafusion-sqlparser-rs) | 0.62.0 | Apache-2.0 | Rust editor/query service |
| TUI | [`termrock`](https://github.com/tailrocks/termrock) | 0.9.0 / exact Git revision | Apache-2.0 | only reusable TUI layer |
| Terminal renderer | [Ratatui](https://github.com/ratatui/ratatui) | 0.30-compatible with pinned TermRock | MIT | through TermRock compatibility tuple |
| Terminal backend/input | [`crossterm`](https://github.com/crossterm-rs/crossterm) | 0.29.0 | MIT | CLI terminal adapter; TermRock `crossterm` feature |
| Ratatui terminal backend | [`ratatui-crossterm`](https://github.com/ratatui/ratatui) | 0.1.2 | MIT | CLI render adapter only; Crossterm 0.29 feature only |
| PTY test harness | [`portable-pty`](https://github.com/wezterm/wezterm) | 0.9.0 | MIT | CLI development dependency only |
| Secret zeroization | [`zeroize`](https://github.com/RustCrypto/utils/tree/master/zeroize) | 1.9.0 | MIT OR Apache-2.0 | core dangerous-local secret buffer only; `alloc` feature |
| Persistence | [`turso`](https://github.com/tursodatabase/turso) local database | 0.7.0 | MIT | one serialized Rust async persistence actor |
| Swift binding | [`uniffi`](https://github.com/mozilla/uniffi-rs) | 0.32.0 | MPL-2.0 | synchronous coarse FFI only |
| Structured diagnostics | [`tracing`](https://github.com/tokio-rs/tracing) | 0.1.44 | MIT | fixed safe fields only |
| Telemetry export | [`opentelemetry-otlp`](https://github.com/open-telemetry/opentelemetry-rust) | 0.32.0 | Apache-2.0 | opt-in OTLP, disabled by default |

TableRock does not directly depend on a separate textarea/widget framework,
`rusqlite`, `libsql`, Turso Cloud sync, Arrow, an Objective-C bridge from Rust,
a local RPC framework, or a second SQL parser. A missing general TUI primitive
is implemented in TermRock.

### Secret zeroization

The core pins `zeroize` 1.9.0 with default features disabled and only `alloc`.
It has no dependencies, supports the workspace Rust version, and gives `Vec`
compiler-resistant zeroing across its full capacity. TableRock uses it only for
the explicitly acknowledged dangerous-local plaintext owner, keeps that owner
non-cloneable, and retains the workspace-wide `unsafe_code = "forbid"` rule.
Reference-only secret variants never contain resolved bytes.

### Phase 1 runtime adoption

The executable shell pins Tokio 1.52.3 with only macros, current-thread runtime,
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

Use rustls with explicit platform/project root handling and client identity.
Invalid-certificate acceptance exists only as a dangerous, visible profile
setting and is never enabled by default.

## ClickHouse

Use official `ClickHouse/clickhouse-rs`. Typed Row APIs handle known catalog
queries. Arbitrary workbench results use `fetch_bytes()` with one selected
self-describing ClickHouse format and convert immediately into owned TableRock
pages. Contract tests cover late errors, query IDs, progress, server
cancellation, nested/nullable/decimal/large integer/binary types, mutations,
compression, and TLS roots.

No hand-written ClickHouse HTTP client exists in the selected architecture.

## Redis

Use `redis-rs/redis-rs` with the minimum async, TLS, and protocol features needed
for the supported standalone deployment. Multiplexed connections handle normal
commands; dedicated connections isolate Pub/Sub and blocking commands. Contract
tests prove byte safety, all SCAN variants, logical database isolation,
post-dispatch cancellation truth, command metadata, TLS, timeout, and reconnect.

Its BSD-3-Clause license and transitive graph are recorded when adopted.

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
