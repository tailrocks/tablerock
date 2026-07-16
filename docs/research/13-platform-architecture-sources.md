# Platform Architecture: Primary-Source Ruling

**Researched:** 2026-07-16
**Scope:** Rust engine, TermRock/Ratatui TUI, Rust-to-Swift boundary, native
macOS presentation, credentials, distribution, and three database adapters.

## Selected architecture

```text
                         tablerock-core
       owned IDs, values, capabilities, commands, events, pages
                                ^
                                |
                         tablerock-engine
        Tokio tasks, policy, sessions, drivers, result storage
                         /              \
                        /                \
       tablerock-tui + termrock       tablerock-ffi
       TEA + Ratatui, in-process      synchronous UniFFI facade
                                              |
                                     SwiftUI + AppKit app
                                     direct notarized release
```

This is the sole implementation path:

- Rust owns all product/database behavior.
- The TUI uses The Elm Architecture and TermRock.
- Crossterm 0.29 is the sole terminal backend/input implementation.
- The TUI calls the engine in-process through owned commands/events/pages.
- The macOS app embeds Rust through synchronous coarse UniFFI bindings.
- SwiftUI owns application structure; AppKit owns catalog/grid/editor controls.
- macOS ships first through direct Developer ID distribution with hardened
  runtime, notarization, and stapling.
- Local-only Turso through the Rust `turso` crate stores profiles, intent,
  preferences, and history.
- `tokio-postgres`, official `ClickHouse/clickhouse-rs`, and `redis-rs` are the
  three client adapters.
- `russh` is the SSH tunnel adapter below those clients.

## Clean-room source boundary

This ruling uses project-owned source and upstream documentation. TablePro,
TablePlus, and Zedis establish only broad workflow existence. Their source,
tests, comments, identifiers, text, assets, screenshots, measurements, colors,
layouts, and key bindings are excluded by the
[clean-room rule](../../AGENTS.md).

TermRock is imported as the shared component library. Jackin is inspected only
as a usage/architecture reference; TableRock imports no Jackin product crate.

## TUI application pattern

TableRock uses The Elm Architecture exactly as fixed in
[07-application-pattern.md](07-application-pattern.md): one root `Model`, one
root `Message` path, deterministic `update`, typed effects, subscriptions merged
into one message queue, and a pure full-frame view.

Ratatui's official [TEA guide](https://ratatui.rs/concepts/application-patterns/the-elm-architecture/)
defines Model, Message/Update, and View, calls the update function the heart of
state change, and requires view predictability/side-effect freedom. Ratatui also
permits Rust to mutate the model in place for practical performance.

Ratatui uses immediate rendering and diffs the current complete buffer against
the previous buffer; each requested draw renders the complete intended frame
([rendering model](https://ratatui.rs/concepts/rendering/under-the-hood/)).
Ratatui documents centralized event catching plus message passing as a scalable
shape ([event handling](https://ratatui.rs/concepts/event-handling/)).

TableRock rules:

- `update` performs no I/O or async work and returns typed effects.
- `view` reads resident immutable state and renders through TermRock.
- terminal, engine, signal, resize, paste, mouse, and requested timer events
  enter the root message path.
- every async completion carries operation ID, session generation, context and
  model revision; stale events are discarded by the root reducer.
- Tokio bounded `mpsc` channels provide backpressure
  ([Tokio `mpsc`](https://docs.rs/tokio/latest/tokio/sync/mpsc/)).
- TermRock owns terminal raw/alternate-screen/mouse/paste lifecycle.

Crossterm's official API supplies cross-platform keyboard, mouse, resize, focus,
paste, raw-mode, alternate-screen, and cursor control. TableRock uses one CLI
`EventStream`; TermRock's scoped Crossterm session owns terminal commands and
restoration
([crate](https://docs.rs/crossterm/latest/crossterm/),
[events](https://docs.rs/crossterm/latest/crossterm/event/),
[terminal](https://docs.rs/crossterm/latest/crossterm/terminal/)).

## TermRock contract

The selected crate is `termrock`. Its project defines product-neutral Ratatui
components, backend-neutral input, semantic styling, stable interaction IDs,
and runtime contracts
([TermRock README](https://github.com/tailrocks/termrock/blob/main/README.md),
[crate README](https://github.com/tailrocks/termrock/blob/main/crates/termrock/README.md)).

Current primitives include ActionBar, Backdrop, ChoiceDialog, DetailTable,
Dialog, DiffView, HintBar, List, MessageDialog, Panel, StatusBar, Tabs,
TextInput, Toast, and Viewport
([component inventory](https://github.com/tailrocks/termrock/blob/main/crates/termrock/COMPONENTS.md)).

Missing reusable primitives—Form, Tree, SplitPane, VirtualGrid, TextArea,
CompletionMenu, Progress, and shared scroll/hit-region behavior—are added to
TermRock `main` before their TableRock screens ship. Each uses neutral names,
borrowed render data, stable IDs, caller-owned policy, docs, lookbook fixtures,
Buffer tests, and Jackin compatibility checks. Database models and policy stay
in TableRock.

## Rust core and engine

`tablerock-core` contains transport-safe owned facts:

- opaque stable IDs and monotonic revisions;
- engine-neutral commands, events, capabilities, errors, and lifecycle states;
- typed values separating null, empty, bytes, text, numeric, temporal,
  truncated, invalid, and unknown values;
- immutable metadata, catalog snapshots, result pages, edit plans, and safe
  summaries;
- secret references and redacted profile metadata, never resolved values.

`tablerock-engine` owns Tokio, database clients, sessions, catalogs, query
workers, result budgets, cancellation, history, persistence, secret resolution,
and safety. Driver rows, statements, sockets, streams, TLS objects, pools, and
raw errors terminate inside adapters.

Shutdown is decide, notify, and await tracked work, following Tokio's
[graceful shutdown guide](https://tokio.rs/tokio/topics/shutdown). A started
`spawn_blocking` task cannot be aborted, so process calls such as `op read` are
short, bounded, timed out, and killed/reaped by explicit process ownership
([`spawn_blocking`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)).

## Cancellation truth

Cancellation uses one operation-ID state machine:

```text
Running
  -> CancelRequested
  -> ServerConfirmed | CompletedBeforeCancel | ClientStopped | Unknown
```

Dropping a TUI/Swift future is never server cancellation. Late terminal events
remain attached to the operation record. Reconnect never retries an ambiguous
write.

PostgreSQL `CancelToken` cannot prove it won a race with normal completion
([`CancelToken`](https://docs.rs/tokio-postgres/latest/tokio_postgres/struct.CancelToken.html)).
Redis documents that dropping a multiplexed request future does not cancel a
request already sent
([`MultiplexedConnection`](https://docs.rs/redis/latest/redis/aio/struct.MultiplexedConnection.html)).
ClickHouse cancellation is implemented and verified through application query
IDs plus official HTTP/`KILL QUERY` behavior
([HTTP interface](https://clickhouse.com/docs/interfaces/http),
[`KILL QUERY`](https://clickhouse.com/docs/sql-reference/statements/kill)).

## Database adapters

### PostgreSQL

Use `tokio-postgres`. Its `Client` is paired with a driven `Connection`, and
`query_raw` yields a `RowStream` suitable for incremental conversion into owned
pages
([crate](https://docs.rs/tokio-postgres/latest/tokio_postgres/),
[`Client`](https://docs.rs/tokio-postgres/latest/tokio_postgres/struct.Client.html),
[`RowStream`](https://docs.rs/tokio-postgres/latest/tokio_postgres/struct.RowStream.html)).

Contract tests cover unknown/custom OIDs, binary values, notices, parameters,
COPY, multiple statements, TLS, cancel races, connection loss, and ambiguous
writes.

### ClickHouse

Use official `ClickHouse/clickhouse-rs`. It supports typed metadata queries,
streaming cursors, progressive inserts, TLS, and byte fetching in a selected
format
([upstream README](https://github.com/ClickHouse/clickhouse-rs/blob/main/README.md),
[`Query`](https://docs.rs/clickhouse/latest/clickhouse/query/struct.Query.html)).

TableRock uses a self-describing result stream for arbitrary workbench queries
and converts it into owned pages. Tests cover nested/nullable/low-cardinality/
decimal/large integer/binary values, late errors, compression, TLS, inserts,
mutations, progress, and query-ID cancellation.

### Redis

Use `redis-rs`. Async multiplexed connections handle ordinary commands;
dedicated connections isolate blocking commands and Pub/Sub
([upstream README](https://github.com/redis-rs/redis-rs/blob/main/README.md),
[crate](https://docs.rs/redis/latest/redis/)).

Tests cover raw byte keys/values, SCAN families, RESP2/RESP3, logical DB
isolation, pipelines, Pub/Sub, blocking commands, TLS, timeouts, reconnect, and
post-dispatch cancellation. Automatic browsing never uses `KEYS`.

Only ClickHouse's selected client is vendor-official. PostgreSQL and Redis use
maintained community clients selected by the project
([PostgreSQL external interfaces](https://www.postgresql.org/docs/current/external-interfaces.html),
[Redis client support](https://redis.io/docs/latest/develop/clients/)).

## SSH transport

Use `russh` client sessions and direct-tcpip channels in one Rust transport
adapter. The official client documentation states that its session handle opens
channels used to tunnel TCP connections
([russh client](https://docs.rs/russh/latest/russh/client/)). Host keys are
verified against the profile's known-hosts policy; authentication uses reviewed
key/agent/password references; keepalive, cancellation, reconnect, and errors
remain bounded and redacted. Database drivers see only the resulting local
stream/endpoint.

## Native macOS and UniFFI

Use SwiftUI's [`App`](https://developer.apple.com/documentation/swiftui/app),
[`WindowGroup`](https://developer.apple.com/documentation/swiftui/windowgroup),
[`Settings`](https://developer.apple.com/documentation/swiftui/settings), and
[commands](https://developer.apple.com/documentation/swiftui/menus-and-commands)
for lifecycle, windows, menus, and preferences.

Use `NSOutlineView` for catalog, `NSTableView` for the virtualized result grid,
and `NSTextView`/TextKit for SQL/command editing. Wrap them through
[`NSViewRepresentable`](https://developer.apple.com/documentation/swiftui/nsviewrepresentable).
Wrapped/custom controls expose complete
[AppKit accessibility](https://developer.apple.com/documentation/appkit/accessibility-for-appkit)
roles, selection, values, and actions.

Use UniFFI-generated Swift over a Rust static library. The exported API is
synchronous and coarse: open, submit, next bounded events, fetch encoded page,
cancel by operation ID, and shutdown. Rust owns Tokio; a non-main Swift actor
polls and decodes, then publishes immutable presentation snapshots to
[`MainActor`](https://developer.apple.com/documentation/swift/mainactor).

UniFFI calls Swift bindings production-quality but documents partial Swift 6
support and async `Sendable` limitations
([Swift bindings](https://mozilla.github.io/uniffi-rs/latest/swift/overview.html)).
Therefore TableRock does not export UniFFI async functions. The generated
headers/module maps/Swift sources and Rust library are packaged as an Apple
[XCFramework](https://developer.apple.com/documentation/xcode/creating-a-multi-platform-binary-framework-bundle).

FFI acceptance proves strict concurrency, deterministic handle destruction,
one transfer per event/page batch, typed/redacted errors, operation-ID
cancellation, worker-to-main actor handoff, panic containment, generated
artifact determinism, signing, allocations, latency, scrolling, and leaks.
Rust panics never cross the boundary
([Rust FFI guidance](https://doc.rust-lang.org/nomicon/ffi.html),
[`catch_unwind`](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html)).

## Credentials and persistence

Persist secret references, never resolved values. Rust owns a versioned
SecretSource model and resolves only the fields needed for Test/Connect.
`op read` resolves 1Password references at runtime
([reference syntax](https://www.1password.dev/cli/secret-reference-syntax),
[multiple accounts](https://www.1password.dev/cli/use-multiple-accounts)).
Do not place database secrets in environment variables by default because
same-user processes may observe process environments
([1Password environment warning](https://www.1password.dev/cli/secrets-environment-variables)).

Direct native distribution permits the same Rust-owned 1Password CLI resolver.
A thin Swift Keychain adapter handles the explicit native Keychain source;
mapping, redaction, lifetime, and database use remain Rust-owned. Apple documents
Keychain as encrypted small-secret storage with application access control
([Keychain Services](https://developer.apple.com/documentation/security/keychain-services)).

Use the official Rust `turso` crate with `Builder::new_local` and one serialized
async persistence actor. Do not enable cloud sync or use the remote `libsql`
client. Turso recommends this crate for local Rust databases and provides native
async I/O
([Rust quickstart](https://docs.turso.tech/sdk/rust/quickstart)).

The researched 0.7.0 release is pre-1.0 and does not yet implement every SQLite
behavior. Pin the proven release, avoid experimental features, validate every
schema/migration statement against the upstream compatibility ledger, and keep
tested independent backups
([upstream status](https://github.com/tursodatabase/turso),
[compatibility](https://github.com/tursodatabase/turso/blob/main/COMPAT.md)). A
failed gate blocks persistence work; it never activates `rusqlite` or `libsql`.

Store profiles, secret references, preferences, saved queries, intent-only
session state, and retention-controlled history. Never store resolved secrets,
result payloads, pending edits, or ambiguous-write retry intents.

Use `tracing` for structured local diagnostics and opt-in
`opentelemetry-otlp` HTTP/protobuf export. Export is disabled by default and the
field schema excludes SQL, Redis arguments, endpoints, credentials, and values
([tracing](https://docs.rs/tracing/latest/tracing/),
[OTLP](https://docs.rs/opentelemetry-otlp/latest/opentelemetry_otlp/)).

## Direct macOS distribution

Ship through Developer ID with hardened runtime, secure timestamp,
notarization, and ticket stapling. Every embedded Rust framework is part of the
signed Release artifact. Apple documents the process in
[notarization](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)
and [hardened runtime](https://developer.apple.com/documentation/security/hardened-runtime).

Release tests run on clean machines and cover signatures, notarization/stapling,
1Password availability/failure, Keychain prompts, file access, network/TLS,
migration, update, crash recovery, and uninstall.

## Evidence gates

| Layer | Required evidence |
|---|---|
| Core | value round trips, revisions, stale-event rejection, paging, redaction, and safety properties |
| Drivers | real-server connect/catalog/query/page/cancel/disconnect/TLS/error/ambiguous-write suites |
| TUI | TEA reducer/effect tests, Buffer tests, TestBackend screens, PTY lifecycle, Unicode/minimum-size/input fixtures |
| UniFFI | Swift 6 compile, ownership/free stress, cancellation races, panic mapping, large pages, leak/allocation/latency evidence |
| Native UI | SwiftUI/AppKit tests, VoiceOver, keyboard/menu/focus/IME, large-grid scrolling, restoration |
| Distribution | signed Release on clean machines, hardened runtime, notarization/stapling, credentials, update/uninstall |

The full quality matrix is in
[32-quality-and-verification.md](32-quality-and-verification.md).

## Trunk-only execution

All implementation occurs through small forward commits directly on `main`.
No branch or pull request exists. A reusable TermRock primitive lands, passes
its own evidence, and is pushed to TermRock `main`; a later TableRock `main`
commit pins that exact revision. Rejected paths and failed evidence are recorded
in decision history because no review-thread archive exists.
