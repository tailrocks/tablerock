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

Use the owned typed value model and immutable pages. UniFFI pages use the
versioned TableRock column metadata/offset/null/type-tag/byte-arena encoding in
one buffer, never one object/call per cell. Arrow is not part of the selected
architecture.

## Catalog and autocomplete

Load database list, selected objects, then selected columns incrementally.
Completion reads immutable schema-index snapshots and receives document
revision, cursor byte offset, dialect, database/schema context, and aliases.
Results combine parser/token context, keywords/commands, catalog candidates,
fuzzy ranking, and bounded recent selections.

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
The later complete durable aggregate composes organization and preferences;
command-level enforcement remains below presentation.

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
