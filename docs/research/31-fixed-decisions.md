# Fixed Architecture Decisions

This file contains the single selected implementation path. A failed evidence
gate blocks work and requires an explicit documented revision; it does not
activate a parallel approach.

| Concern | Fixed decision |
|---|---|
| Product | standalone TableRock repository and product |
| Scope | PostgreSQL, ClickHouse, and Redis only |
| First UI | Rust CLI/TUI using TermRock and Ratatui |
| TUI pattern | The Elm Architecture; one root Model/Message/Update/Effect/View path |
| Shared widgets | TermRock only; missing neutral primitives land there first |
| Terminal backend | Crossterm 0.29 through TermRock's adapter plus one CLI EventStream |
| PostgreSQL | `tokio-postgres` plus rustls adapter |
| ClickHouse | official `ClickHouse/clickhouse-rs` |
| Redis | `redis-rs/redis-rs`, standalone deployment first |
| SQL analysis | `sqlparser` tokens plus last-known-valid AST and catalog index |
| Persistence | bundled SQLite through `rusqlite` on a dedicated Rust worker |
| Credentials | Rust-owned SecretSource; 1Password CLI primary, prompt/Keychain/acknowledged plaintext explicit sources |
| Native UI | SwiftUI application shell with NSOutlineView, NSTableView, and NSTextView adapters |
| Native bridge | embedded Rust static library through synchronous coarse UniFFI |
| Native distribution | direct Developer ID, hardened runtime, notarization, and stapling |
| SSH transport | Rust `russh` adapter below all three database clients |
| Telemetry | local `tracing`; opt-in redacted OTLP export, disabled by default |
| Results | bounded immutable TableRock pages; versioned columnar byte arena across UniFFI |
| Safety | Rust-enforced ReadOnly/ConfirmWrites plus specific destructive confirmation |
| References | concepts only; no copied expression/source |
| Delivery | direct forward commits on `main`; no branches or pull requests |
| Automation | AI/MCP/external-agent database access excluded |

## Product and profile identity

TableRock remains the working/product name. Trademark, category, domain, package
namespace, and distribution checks must pass before public release.

Profiles are global definitions with stable IDs. Optional project bindings
control visibility/order only and never grant database or agent authorization.

## Secret model

`tablerock-core` owns a versioned `SecretSource` reference model. Rust resolves
1Password references with bounded `op read` calls. Prompt-on-connect is
transient. A thin Swift adapter services explicit Keychain references for the
native client. Plaintext is retained only as an acknowledged dangerous local
testing source. Resolved bytes never enter stable state, FFI events, logs,
history, telemetry, or crash reports.

The secret abstraction stays TableRock-local. Sharing it requires a separate
security decision; it is not part of TermRock.

## Persistence

Use `rusqlite` with bundled SQLite, migrations, foreign keys, WAL mode, busy
timeout, integrity checks, and one dedicated Rust worker. Persist profiles,
secret references, organization, saved queries, preferences, intent-only
restoration, bounded history, and support facts. Never persist result payloads,
resolved secrets, pending edits, or ambiguous-write retry intent.

## Server support

The public support claim equals the exact real-server matrix continuously run in
CI. Each engine adopts its current stable production line and one preceding
production line when the Phase 2 contract suite passes. A server leaves support
when its matrix row is removed with migration/release documentation. No broader
semantic minimum is implied.

## Result budgets and encoding

Start with 500-row pages, a 10,000-row arbitrary-query cap, and independent byte
and process-memory budgets. Phase 2 measurements may revise the numeric values
before user release, but there is no unlimited mode.

Rust pages use immutable column metadata, offset/null/truncation arrays, value
kind tags, and one byte arena. UniFFI transfers the versioned page as one
`Vec<u8>` plus safe envelope metadata. Swift validates bounds and decodes away
from `MainActor`; there is no call/object per cell.

## ClickHouse arbitrary results and writes

Use the official client's `fetch_bytes("RowBinaryWithNamesAndTypes")` path.
TableRock parses the self-describing names/types and decodes rows into owned
typed pages. Typed official-client rows remain for known catalog queries.

Writes begin with reviewed batch INSERT. UPDATE/DELETE use ClickHouse mutations
only after operation/query identity and mutation polling tests pass. UI always
shows asynchronous, non-transactional outcomes.

## Redis writes

Support string, hash, set, sorted-set, list, and TTL changes with explicit type
operations. Streams are read-only in the first program. TTL preservation or
replacement is an explicit part of every change plan. Module values are
inspectable/read-only unless their exact command/type contract is added later.

## SQL/editor path

Use `sqlparser` for PostgreSQL/ClickHouse tokens and valid ASTs, with
dialect-aware statement boundaries and token fallback for incomplete input.
Redis uses official command metadata plus its own command tokenizer. Completion
is revisioned against editor text, context, and catalog generation.

TermRock owns the neutral multiline `TextArea` and `CompletionMenu`; TableRock
owns parser services, database candidates, diagnostics, and execution.

## TermRock extension rule

Every missing product-neutral form, tree, split, grid, editor, completion,
progress, scroll, focus, input, or hit-region primitive is added to TermRock
`main` first. The API must be neutral and reusable by TableRock, Jackin, and
future products, with borrowed render data, stable IDs, lookbook stories, docs,
Buffer/interaction tests, performance evidence, and Jackin compatibility.

Database catalogs, values, query semantics, edit plans, secret handling, and
safety never enter TermRock.

## Terminal backend

Crossterm is the only terminal backend/input library. `tablerock-cli` owns one
Crossterm `EventStream`, maps events into root TEA messages, and enables
TermRock's Crossterm session. TermRock alone owns raw mode, alternate screen,
mouse/paste modes, cursor, and restoration. No widget or reducer emits a
terminal command directly.

## Native architecture

The native app embeds the Rust engine with synchronous coarse UniFFI calls.
Rust owns Tokio. A non-main Swift actor submits commands, polls bounded events,
fetches encoded pages, and sends operation-ID cancellation. Immutable
presentation snapshots move to `MainActor`.

SwiftUI owns application/windows/commands/settings. `NSOutlineView` owns catalog
presentation, `NSTableView` owns the large grid, and `NSTextView`/TextKit owns
native editing/IME/find. Swift contains no driver, parser safety, mutation,
history, reconnect, redaction, or result authority.

Ship direct through Developer ID, hardened runtime, notarization, and stapling.
Mac App Store distribution and a helper daemon are not in this program.

## SSH and cloud transport

SSH tunneling uses one Rust `russh` adapter below the database clients. It owns
host-key verification/known-hosts, agent/key/password authentication, local
forwarding, keepalive, cancellation, reconnect, and redacted errors. Database
drivers receive only the established local stream/endpoint. Cloud-provider
proxy and identity integrations are excluded from this program.

## Telemetry

Use `tracing` locally. Optional OpenTelemetry export uses OTLP and the fixed safe
schema of IDs, engine, safe codes, durations, counts, and state transitions.
Export is disabled by default. SQL, Redis arguments, credentials, endpoints, and
cell values are never exported.

## Safety and excluded automation

Profiles select ReadOnly or ConfirmWrites. Every destructive operation requires
specific reviewed confirmation below presentation. AI query generation, AI
chat, MCP, and external-agent database access remain excluded and have no
roadmap phase.
