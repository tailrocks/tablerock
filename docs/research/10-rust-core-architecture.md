# Rust Core Architecture

## Initial crates

### `tablerock-core`

Stable language shared by engine and clients:

- opaque profile/session/context/tab/query/result/row/mutation IDs;
- engine, TLS, safety, and connection-source values without resolved secrets;
- capabilities and engine-specific facts;
- catalog nodes/snapshots;
- owned typed values, columns, row batches/pages, summaries;
- commands, events, errors, revisions, edits, and restoration records.

It has no Ratatui, AppKit, concrete database client, socket, task handle, or
borrowed client row. Types are bounded, owned, and UniFFI-facade safe.

### `tablerock-engine`

Runtime ownership:

```text
src/
  engine.rs
  session.rs
  profile.rs
  secret.rs
  catalog.rs
  query.rs
  result_store.rs
  edit.rs
  history.rs
  redaction.rs
  drivers/{postgres,clickhouse,redis}.rs
```

It owns Tokio tasks/cancellation, sessions, driver adapters, catalog caches,
query coordination, bounded results, edits, storage, reconnect/shutdown, and
redacted telemetry. Keep drivers as modules in `tablerock-engine`; do not create
independently released driver crates.

### `tablerock-tui`

TableRock presentation model, messages, pure updates, effects-as-data,
subscriptions, views, and product compositions under The Elm Architecture. It
consumes `termrock` but no concrete driver. Messages carry IDs and bounded
snapshots/pages.

### `tablerock-cli`

Binary, command parsing, configuration/data paths, one Crossterm EventStream,
TermRock terminal session, signals, effect execution, noninteractive text/JSON
output, secret adapter, and telemetry initialization.

Later `tablerock-ffi` exposes the synchronous coarse UniFFI facade after the
terminal contracts stabilize across all three engines.

## Dependency direction

```text
termrock <------------------ tablerock-tui
                                  |
                                  v
                           tablerock-core
                                  ^
                                  |
                          tablerock-engine

tablerock-cli ------------> tablerock-tui + tablerock-engine
tablerock-ffi ------------> tablerock-engine + tablerock-core
```

No runtime-to-presentation edge is allowed.

TableRock may use Ratatui types exposed by TermRock for composition, but it does
not create a competing interactive widget layer. A missing product-neutral
primitive is implemented and proven in TermRock first. TableRock owns only the
database-specific model, formatting, policy, and screen composition over those
primitives.

## Driver boundary

A small shared lifecycle exposes engine/capabilities, connect, catalog,
execute, cancel, and shutdown. Execution/catalog requests are enums with typed
PostgreSQL, ClickHouse, Redis, object-page, metadata, and mutation variants.

Constraints:

- bounded cancellation-aware event sinks;
- redacted errors preserve safe engine code/severity/position;
- client row/socket types never cross the adapter;
- write plans are typed parameters/commands, not executable preview strings;
- reconnect never automatically retries ambiguous writes.

The Phase 2 object-safe adapter seam returns explicit boxed `Future` values;
native `async fn` methods are not dyn-compatible. `DriverSession` exposes
engine identity, bounded page-stream start, explicit cancellation dispatch, and
consuming shutdown. `DriverPageStream` returns immutable core pages. Concrete
client sessions, rows, cursors, and errors remain behind the implementation.
Cancellation reports `Unsupported` until an adapter can map the supplied
operation identity to a real server request; it never substitutes task drop.
The bounded `DriverRuntime` maps core operation identities to engine-owned
Tokio tasks, type-erased sessions, single-slot cancel channels, latest-state
stop signals, and bounded event delivery. Control remains responsive under
output backpressure. Task
exit distinguishes completion, client stop, and safe failure; the core remains
the sole lifecycle authority. Unknown/unsupported/request-sent cancellation is
preserved without manufacturing server confirmation.

## Commands, events, and revisions

Commands represent operator intent:

```text
TestProfile / SaveProfile / Connect / Disconnect
SelectContext / RefreshCatalog / OpenObject
Execute / Cancel / FetchPage
StageMutation / ReviewMutations / ApplyMutations / DiscardMutations
```

Events carry profile-test outcome, session/catalog/tab snapshot, query progress,
result changes/pages, change review, and redacted failure. Mutable aggregates
have monotonic revisions; stale pages/completions/events are discarded.
Cumulative consecutive progress may coalesce in the bounded per-operation
queue. Required-event overflow or producer sequence loss emits one resync
marker; it never silently drops lifecycle truth.

`ServiceCoordinator` owns a finite set of overlapping operation records,
canonical command identity, parent-scope containment, lifecycle cursors, event
queues, cancellation requests, terminal retirement, and drain shutdown. It
never equates a cancel request or process shutdown with a server-confirmed
outcome. Its finite hierarchical scope registry owns aggregate revisions and
rejects unknown, stale, or future command expectations before operation
capacity or driver state can change. It also rejects stale in-flight progress
without suppressing lifecycle and terminal outcome truth.

Each operation owns a finite set of opaque subscriptions. Events fan out through
independent bounded queues; a slow subscriber receives its own resync marker and
cannot block or degrade a current subscriber. Late subscribers start current
only at the exact authoritative sequence, otherwise they receive resync. An
operation cannot retire while any subscription handle or queued event remains.

## Session ownership

One engine owns profiles, live sessions, queries, catalogs, results, and
history. Session state is explicit:

```text
Disconnected -> Connecting -> Connected
       ^              |            |
       +----------- Failed <- Reconnecting
```

Transitions carry reason, attempt, time, and required operator action. Read-only
metadata may retry under a bounded policy; user queries require explicit retry
unless the driver proves no request reached the server.

## Result store

```text
driver stream -> bounded channel -> assembler -> immutable batches
                                             -> page projection
```

The store enforces row/byte caps, tracks completion/truncation/cancellation,
and evicts unpinned batches under a memory budget. Table tabs page from the
server and cache near the viewport. Arbitrary queries stream to a configured
cap. The status distinguishes row-cap, byte-cap, cancelled, and failed.

Result identities are opened explicitly before page admission. A page cannot
implicitly create or revive a result. The core store rejects foreign engines,
stale/future revisions, duplicates, and overlapping resident ranges; a newer
opened revision invalidates every old page even when pinned. Global LRU eviction
is deterministic and transactional, and pinned capacity fails admission without
mutating resident state. Page-count and actual owned buffer-capacity limits are
both finite; result slots close explicitly.

Use the owned typed value model and immutable pages. UniFFI pages use the
versioned TableRock column metadata/offset/null/type-tag/byte-arena encoding in
one buffer, never one object/call per cell. Arrow is not part of the selected
architecture. Database-native containers use the dedicated bounded
`Structured` value kind, not ordinary text or an untyped JSON bypass.

## Mutation plans

Rust owns each immutable mutation plan: opaque identity, operation scope,
revision, typed engine target, exact changes, finite bounds, and truthful
execution model. Execution never reparses descriptive preview text. Empty or
cross-engine changes, duplicate fields, null locators, invalid expirations, and
`Invalid`, `Unknown`, truncated, or `Structured` values fail before an adapter
can observe the plan.

PostgreSQL plans are atomic transactions. ClickHouse inserts are progressive
and non-transactional, while updates/deletes are asynchronous mutations and
cannot share an insert plan. Redis string/key/expiration changes are sequential
with no rollback claim.

Review consumes the exact plan into a non-cloneable reviewed value.
Authorization consumes that value and rejects expired tokens or scope/revision
drift. Any edit therefore requires a new review. A bounded Rust-owned registry
retains reviewed plans behind opaque tokens and removes authority before every
authorization attempt. The later service/UniFFI seam must expose this registry,
not serialize an authority-bearing plan into Swift.

## Catalog and autocomplete

Load database list, selected objects, then selected columns incrementally.
Completion reads immutable schema-index snapshots and receives document
revision, cursor byte offset, dialect, database/schema context, and aliases.
Results combine parser/token context, keywords/commands, catalog candidates,
fuzzy ranking, and bounded recent selections.

The core catalog is a bounded immutable preorder forest with stable opaque node
IDs, explicit parent IDs/depth, aggregate text limits, lazy child state, and
safe diagnostics for failed loads. Kinds preserve PostgreSQL database/schema/
object/column, ClickHouse database/object/column, and Redis logical-database/
projected-namespace/key semantics. Snapshot construction rejects fake
cross-engine hierarchies, malformed trees, misplaced engine types, and unsafe
failure state. A scope/engine/revision cursor accepts only the immediate next
snapshot and requires resync on revision gaps.

## Profiles and credentials

Persist a documented versioned profile schema with stable IDs, engine,
endpoint/default context, value source per property, TLS, safety, timeouts, and
preferences. High-churn history/cache/restoration use local-only Turso through
one serialized Rust async persistence actor. Do not persist results or pending
edits.

```text
ConnectionValue
  Literal(non-secret)
  SecretSource
```

The public constructor rejects `Literal` for passwords, TLS client private
keys, and their passwords. All properties may use `SecretSource`, allowing
metadata-only 1Password/environment/prompt/Keychain/dangerous-local sources
without creating a second credential model. Versioned property sets reject
duplicates and bound every literal before persistence or adapter boundaries.

An immutable schema-versioned profile connection sub-snapshot owns stable identity/revision,
engine, redacted bounded name, property set, TLS policy, safety mode, and finite
limits. Every engine requires host and port sources. TLS configuration is a
closed validated state; profiles select only `ReadOnly` or `ConfirmWrites`.
The complete durable aggregate composes organization and preferences;
command-level enforcement remains below presentation.

The baseline aggregate gives only saved profiles a non-constructible
persistence token; temporary profiles are structurally memory-only. It bounds
group/tags/favorite/order, stores only bounded
reconnect/context/page preferences, and validates non-consuming monotonic
replacement before the persistence actor repeats the CAS transactionally.
The list boundary is a separate least-data projection: immutable pages contain
at most 100 redacted identity/organization/safety/source-fact summaries and an
opaque keyset cursor. List adapters never load secret payload columns or reuse
full aggregates as presentation rows.
Every continuation owns its closed filter scope; changing an engine, favorite,
bounded group, or bounded tag filter requires a fresh first page instead of
silently reusing a cursor from a different ordered set. Cursor diagnostics keep
owned labels redacted.
Optional search terms use one versioned NFKC/full-case-fold/NFKC contract and
join the owned cursor scope. The durable profile population is capped at 10,000
so normalization work is finite without persisting derived keys that can drift
from authoritative labels.
List endpoints contain only core-validated literal host/port display values or
an unresolved secret-source marker. Adapter projections structurally exclude
secret reference payloads; endpoint diagnostics remain length-only.

`OpRef` stores stable 26-character account/vault/item object IDs, a bounded
section/field ID path, and a display breadcrumb. A metadata-only picker suggests reviewed mappings for host,
port, database/index, user, password, and TLS fields. A CLI-owned resolver runs
bounded account-pinned `op read` calls only during Test/Connect. Resolved values
never enter core snapshots or telemetry.

The official reference format and CLI read behavior are documented in
[1Password secret references](https://www.1password.dev/cli/secret-references).

Do not depend on `jackin❯` internal secret modules. Secret handling remains a
TableRock-local Rust service.

## Safety and telemetry

- safety mode participates in every write request;
- prepared/bound values and dialect-owned identifier quoting;
- operation-specific destructive confirmation tokens;
- Redis unknown commands classified as writes;
- timeout/result budgets below presentation;
- redaction before UI, logs, telemetry, or diagnostics;
- default telemetry records IDs, engine, safe codes, durations, counts, and
  state transitions, not SQL or values;
- `tracing` is local; opt-in OpenTelemetry exports only the fixed safe schema
  over OTLP. Export is disabled by default and never uses a Parallax-specific
  database API.

## Testing

- pure capability/value/quoting/reducer/paging/redaction tests;
- one driver contract harness over pinned PostgreSQL, ClickHouse, Redis;
- cancellation/disconnect/reconnect/stale-event/ambiguous-write races;
- 1Password rename, signed-out, partial mapping, multi-account, timeout, and
  redaction cases;
- TUI conformance at minimum/normal/wide sizes and all data/error states;
- performance harness for first row, throughput, memory, scrolling,
  cancellation, catalog completion, and Redis scans.
