# Delivery Plan

## Delivery model

This is a trunk-only, evidence-gated program. Work directly on `main`; never
create a branch or pull request. A phase is a dependency map, not permission to
batch months of changes into one commit. Each approved checkpoint is a small,
buildable, documented, tested, DCO-signed forward commit.

Phase 0 was approved on 2026-07-16. This plan now authorizes dependency-ordered
implementation checkpoints; approval alone adds no application code or
dependency and makes no capability claim.

## Program invariants

1. Only PostgreSQL, ClickHouse, and Redis are product engines.
2. TablePro/TablePlus/Zedis establish broad workflows only; source, expression,
   screenshots, measurements, colors, text, assets, identifiers, tests, and key
   bindings are excluded.
3. TermRock is the only reusable interactive TUI component layer. Missing
   neutral primitives land on TermRock `main` first and remain reusable by
   TableRock, Jackin, and other consumers.
4. TableRock database compositions and policy stay local; Jackin remains a
   read-only architecture/usage reference.
5. One Rust application service owns domain/database behavior for both clients.
6. TUI update/render performs no I/O. Effects execute I/O and return typed,
   revisioned events through bounded channels.
7. Results cross UI/native boundaries in immutable batches/pages, never through
   per-cell calls or driver-owned rows.
8. Secrets, SQL text, Redis arguments, and cell values are absent from default
   logs, telemetry, diagnostics, crash reports, and bridge messages.
9. Read/write safety and redaction are enforced below presentation.
10. Cancellation reports observed outcomes; reconnect never retries an
    ambiguous write.
11. Schema/contract changes include versioning, migration, fixtures, tests, and
    documentation in the same checkpoint.
12. Every checkpoint leaves buildable, truthful behavior and updates the
    functional-parity ledger.

## Checkpoint exit rule

Before each future implementation commit:

- approved research names the behavior and dependencies;
- source provenance and licenses are recorded;
- failure, cancellation, security, and unsupported states are specified;
- relevant local/real-server/render/performance evidence passes;
- documentation and support claims match the result;
- the commit is created and pushed directly on `main` following
  [33-main-branch-delivery.md](33-main-branch-delivery.md).

If a checkpoint fails, record evidence and make the next forward repair. Do not
hide failure by rewriting published `main` history.

## Phase 0 — decision freeze and spikes plan

### Deliver

- approve scope, clean-room policy, parity vocabulary, and exclusions;
- approve core ownership, engine adapters, TermRock boundary, and TEA TUI;
- approve Rust command/event/page/value/cancellation/redaction vocabulary;
- clear working name, package namespaces, license policy, and attribution;
- select the database/server matrix and terminal/macOS floors;
- approve the SecretSource variants and direct notarized distribution;
- approve local-only Turso, result-budget measurements, and synchronous UniFFI;
- verify every selected path in
  [31-fixed-decisions.md](31-fixed-decisions.md) is reflected consistently in the
  roadmap, quality gates, and source ruling.

### Exit evidence

Research reviewers can trace every roadmap feature to the parity ledger and
every dependency/architecture decision to primary sources or a planned spike.
Only then may code/dependency checkpoints begin.

## Phase 1 — TermRock substrate and empty TUI

### TableRock checkpoints

- pin one full TermRock Git revision and compatible Ratatui/Rust tuple;
- pin Crossterm 0.29 in that tuple, enable one CLI `event-stream`, and use
  TermRock's Crossterm session as the only terminal lifecycle owner;
- create minimal CLI, terminal session, panic/error restoration, and signal path;
- establish Model/Message/Update/Effect/Subscription/View module boundaries;
- establish bounded engine-event subscription and dirty/full-frame rendering;
- implement wide/medium/narrow shell, minimum-size state, focus order, action
  discovery, keyboard/mouse parity, and non-color state cues;
- add reducer, direct `Buffer`, `TestBackend`, and PTY harnesses.

### TermRock checkpoints

- T0: verify current runtime/session/widgets from a minimal TableRock consumer;
- T1: add neutral `Form`, `Tree`, `SplitPane`, and required scroll/hit-region
  APIs on TermRock `main`;
- document caller-owned policy, add lookbook stories and deterministic previews,
  test Unicode/minimum rectangles/input/focus/mouse, and verify Jackin remains
  compatible before TableRock advances its exact pin.

### Exit evidence

The shell renders and restores the terminal on normal, error, signal, and panic
paths; reducers are deterministic; components perform no I/O; no duplicate
generic widget layer exists in TableRock.

## Phase 2 — Rust core, persistence, and driver feasibility

### Shared contracts

- stable opaque IDs and aggregate revisions;
- engine capability facts and explicit unsupported states;
- owned typed values distinguishing null/empty/binary/truncated/unknown;
- immutable column metadata, pages/batches, catalog snapshots, and edit plans;
- commands/events with operation/session/context/revision identity;
- bounded result store, eviction, resync, cancellation, shutdown, and safe error
  taxonomy;
- profile/secret-reference model with no resolved secret in stable state.

### Storage proof

Prove local-only `turso` through `Builder::new_local`, default features disabled,
and one serialized async persistence actor. Cover startup, migrations, ordinary
transactions, foreign keys, indexed bounded history, single-owner behavior,
crash/integrity/recovery, backup/restore, package size, license, and macOS/
terminal deployment. Publish compatibility, migration, and recovery rules before
profile persistence. Never add `rusqlite`, `libsql`, or cloud sync as fallback.

### PostgreSQL spike

Using `tokio-postgres`, prove driven connection ownership, arbitrary/unknown
types, incremental `RowStream`, TLS roots/client identity, notices, parameters,
multiple statements, COPY, cancellation races, connection loss, and ambiguous
writes.
The full service path distinguishes PostgreSQL cancellation request delivery
from SQLSTATE-confirmed server cancellation. Verified custom roots, independent
server name, client identity, downgrade rejection, and TLS cancellation pass on
both pinned lines. The completion race now passes plain and required-mTLS
transports: SQLSTATE `57014` proves cancellation while a late successful cancel
after `SELECT 1` preserves normal-completion truth. A bounded synchronization
barrier consumes a pending late cancel before releasing the session, preventing
it from striking the next operation. The remaining
protocol/failure matrix stays required.
Force-stopping both pinned server lines before the cancel socket opens now proves
redacted cancellation-transport failure and terminal session connection loss,
without a false server-cancel outcome.
Prepared UTF-8 text, int8, binary-with-NUL, and boolean parameters now pass both
pinned lines through the bounded typed stream seam without exposing client types.
Declared text NULL and `int4[]` parameters also pass both pinned lines; arrays
retain bounded unknown bytes and exact engine type identity until structured
decoding is implemented.
`int4range` and anonymous-record results pass both pinned lines as bounded
unknown binary values with exact engine type and original-byte length truth;
large `bytea` remains binary. JSON and JSONB now advance to deterministic
compact bounded `Structured` projections with arbitrary-precision number and
malformed/version-invalid truth. Other structured decoding and strict
pre-driver transport allocation remain required.
Finite arbitrary-precision PostgreSQL numeric values, declared scale/trailing
zeros, scaled zero, NaN, and infinities now pass both pinned lines as exact core
`Decimal` values. Malformed wire values remain `Invalid`; valid projections over
the cell limit remain typed bounded `Unknown`.
Rust now drives asynchronous PostgreSQL messages directly. Both pinned lines
prove bounded redacted notices, UTF-8-safe truncation, ordered retention, and
explicit overflow without protocol backpressure.
Optional notice detail and hint remain independently bounded, preserve absence
and truncation truth, and stay redacted from Debug.
Both pinned lines now prove ordered CREATE/INSERT/UPDATE/SELECT outcomes with
exact command/query kind and row counts while typed rows remain on the binary
extended-query path.
Both pinned lines now prove pull-driven bounded COPY OUT chunks and
backpressured bounded COPY IN with exact byte facts, server-confirmed import
rows, explicit limit failures, malformed-input recovery, and payload-redacted
Debug. Product file effects, cancellation, progress, and arbitrary reviewed
COPY plans remain open.
Both pinned lines now prove that a post-dispatch response timeout remains an
unknown write outcome even when the server later commits exactly one row. The
original session drains and remains usable; no retry occurs. Transport-loss and
transaction-commit ambiguity remain open.
Both pinned lines also prove explicit COMMIT ambiguity through a deferred
constraint trigger: response observation ends during commit, exactly one row is
later durable, the original session drains, and the transaction never replays.
Both lines now also prove activity-gated transport loss during COMMIT: old
sessions terminate, same-directory restart refreshes endpoint facts, rollback
is observed, and no replay occurs. Other transport-loss timings remain open.
The same matrix now passes required custom-root TLS and client identity on both
lines: old TLS sessions terminate, plaintext recovery is rejected, endpoint
facts refresh, mTLS is revalidated, rollback remains observable, and no replay
occurs.

### ClickHouse spike

Using official `ClickHouse/clickhouse-rs`, prove a self-describing arbitrary
`RowBinaryWithNamesAndTypes` result path, bounded streaming, late HTTP errors,
nested/nullable/decimal/large integer/binary values, compression, TLS,
progressive insert boundaries, query-ID progress/cancellation, and mutation
identity.
The service now binds active query IDs and requires synchronous `finished`
evidence from `KILL QUERY`; remaining progress, mutation, TLS, and failure-race
matrix evidence stays required.

### Redis spike

Using `redis-rs`, prove raw bytes, SCAN families, logical DB isolation,
RESP2/RESP3, TLS, pipelines/partial failures, Pub/Sub and blocking isolation,
timeouts/reconnect, and the difference between client stop and server outcome.
The shared service now uses an isolated blocking-operation connection and
requires both `CLIENT UNBLOCK` reply `1` and the operation-side server error
before reporting server-confirmed cancellation.
The supported-line RESP2/RESP3 matrix now proves per-command pipeline outcomes,
continued execution after a runtime response error, and `MULTI`/`EXEC`
no-rollback truth. It also proves exact missing/persistent/finite-millisecond
key TTL facts through one Rust-owned contract. Bounded binary HSCAN, SSCAN, and
ZSCAN pages now pass both supported lines under RESP2 and RESP3. Verified
custom-root TLS, optional mTLS identity, ACL authentication, and bounded
initial authentication-stop behavior also pass that supported-line/protocol matrix.
Binary-safe pattern subscriptions now pass the supported-line/protocol matrix;
bounded reconnect/resubscription now emits an explicit delivery-gap page before
restored messages. TLS/mTLS/ACL channel and pattern composition now passes the
same matrix. Restricted-channel server denial is measured, but adapter rejection
remains required because redis-rs 1.4.0 erases the Pub/Sub setup error reply;
administrative preflight and a private protocol path are forbidden substitutes.
Password rotation followed by confirmed user-connection termination now proves
bounded redacted authentication failure on the next future operation across the
same matrix. Active channel and pattern subscriptions now also terminate with
bounded redacted authentication failure after confirmed credential rotation and
user-connection termination. TLS and required-mTLS channel/pattern streams prove
same-endpoint server replacement, ordered discontinuity before restored
delivery, and prompt cancellation across the supported matrix.
Untrusted replacement identities and rotated replacement ACL credentials also
terminate as distinct bounded redacted failures without a false recovery page.
Remaining failure races stay required. Reviewed
single-command TTL mutation now proves exact-once
authorization, applied/not-applied truth, and unknown post-dispatch outcomes.
Dedicated bounded binary Pub/Sub streams pass
both supported lines under RESP2 and RESP3 without changing the shared command
connection; cancellation is explicitly client-stop. The supported
matrix proves bounded response timeout and confirmed-drop future-call reconnect
without automatic command replay, plus Redis's stable-throughout and
absent-throughout guarantees during concurrent
SCAN/HSCAN/SSCAN/ZSCAN mutation; transient membership remains intentionally
undefined and duplicates remain legal.

### Exit evidence

One contract harness runs overlapping operations against real pinned servers
(the first simultaneous PostgreSQL/ClickHouse/Redis proof is recorded in
[`126-phase-2-three-engine-overlap.md`](126-phase-2-three-engine-overlap.md));
engine differences are capabilities, not fake normalization; measured first-row,
throughput, cancel, allocation, and memory facts define initial budgets;
rejected dependencies and unsupported claims are recorded.

## Phase 3 — profiles and connection experience

### Deliver

- searchable connection list, engine picker, grouping/tags/favorites/order;
- create/edit/duplicate/remove and URL-to-reviewable-draft flows;
- saved and temporary profiles with stable IDs and versioned migration;
- capability-driven General, TLS, Safety, and engine-specific form sections;
- 1Password `op://` mapping per needed field, prompt-on-connect, environment
  references, Keychain reference shape, and acknowledged dangerous plaintext;
- Test returning server identity/version, TLS outcome, elapsed time, and safe
  diagnostics without saving;
- Connect/disconnect, context switch, health, bounded reconnect/backoff, and
  authentication stop conditions;
- read-only, confirm-write, and destructive confirmation modes enforced in Rust.

### Exit evidence

All engines pass local and verified TLS fixtures; picker/search never resolves
secrets; Test/Connect resolves only requested fields; temporary connection
leaves no durable secret/profile; reconnect cannot repeat an ambiguous write;
profile forms use TermRock `Form`/`Tree` rather than local substitutes.

## Phase 4 — PostgreSQL read-only vertical slice

### Deliver

- session/result lifecycle and revisioned connection state;
- incremental databases/schemas/tables/views/catalog with subtree refresh;
- object preview/pin tabs and structure/raw DDL projection;
- bounded table browsing and arbitrary SQL streaming;
- typed cells, unknown fallback, full value inspector, binary/JSON views;
- explicit queued/running/streaming/complete/cancel-requested/cancelled/failed/
  disconnected states;
- server sorting/filtering with typed plans and parameterized identifiers;
- table/result paging, unknown totals, truncation, status, and safe errors;
- TermRock T2 `VirtualGrid` plus product-local `DataGridModel` composition.

### Exit evidence

First rows render before completion; stale pages/events cannot cross reconnect,
context, or query revisions; resident scrolling performs no I/O/full-result
allocation; caps are exact; unknown values remain inspectable but non-editable;
PostgreSQL cancellation reports its observed race outcome.

## Phase 5 — shared workbench experience

### TermRock T3

Add neutral `TextArea` and `CompletionMenu` with grapheme-safe editing,
selection, undo/redo, line numbers, search, scroll, paste, external spans/
diagnostics, geometry clamping, stable candidate IDs, lookbook/Buffer tests, and
Jackin compatibility. Parser, ranking, database policy, and execution remain in
TableRock.

### TableRock deliverables

- multiline SQL/Redis editor projection and external syntax diagnostics;
- selection/current-statement execution with incomplete-input behavior;
- revisioned catalog/keyword/function/type/command completion;
- bound parameters, find/replace, formatting, and raw/structured explain base;
- grid widths/hide/order/format, range/row selection, TSV/CSV/JSON/Markdown
  copy, and resident/server filtering distinction;
- independent query/result tabs and multi-statement outcome states;
- query files with atomic save/external-change handling;
- bounded/searchable history with configurable SQL retention/private mode;
- saved queries, favorites, quick switcher, preferences, and intent-only
  restoration without persisted results or pending writes.

### Exit evidence

Incomplete SQL/commands never panic; statement and parameter tests resist
injection; stale completions reject by text/context revision; editor/grid work
at minimum/normal/wide sizes and Unicode/IME-like paste cases; history and
restoration honor retention/redaction policy.

## Phase 6 — PostgreSQL writes and administration

### Deliver

- editability proof from base object, stable key, permissions, and result shape;
- typed cell editors and insert/update/delete mutation reducer;
- visible staged changes, undo, discard, review, typed operation preview;
- parameterized transaction apply, conflict handling, rollback, generated-value
  reconciliation, and durable unknown-outcome record;
- foreign-key navigation, table operations, refresh/rename/truncate/drop gates;
- activity/dashboard snapshots with permission-aware cancel/terminate;
- PostgreSQL structure/index/constraint facts and first reviewed DDL operations.

### Exit evidence

Hostile identifiers/values cannot alter operation structure; a multi-change apply
is all-or-rollback where PostgreSQL guarantees it; joins, aggregates, missing or
ambiguous keys stay read-only; refresh/quit cannot silently discard edits;
ambiguous writes never retry.

## Phase 7 — ClickHouse complete slice

### Deliver

- databases/tables/views/dictionaries and engine-specific structure/DDL;
- arbitrary dynamic results without compile-time row structs;
- complex value projection and bounded table/query pages;
- query ID, progress, partial errors, cancel request and observed server outcome;
- explain variants, parts, batch insert, mutation creation/status/cancel where
  official capabilities and permissions prove it;
- gated UPDATE/DELETE and engine operations with asynchronous outcome language.

### Exit evidence

The official client remains the only transport; an upstream gap blocks the
affected capability. HTTPS and the complex-value corpus pass; partial data plus
late error remains visible; client stop and server cancellation remain distinct;
no mutation is labeled transactional or complete before observed server state
proves it.

## Phase 8 — Redis complete slice

### Deliver

- isolated logical database selection and reconnect-safe context;
- SCAN cursor browser, namespaces, filtering, refresh, and unknown totals;
- string/hash/list/set/sorted-set/stream inspection with byte-safe key/value and
  JSON/text/hex projections;
- TTL state, bounded value loading, explicit truncation, and current INFO view;
- command editor/completion, typed result projection, pipelines, and explicit
  blocking/PubSub isolation;
- staged type-specific edits, TTL preservation/change, review, and destructive/
  unknown-command gates.

### Exit evidence

Browsing never uses `KEYS`; changing keyspaces do not produce false exact
totals; bytes round-trip; large values stay bounded; logical DBs cannot race;
MULTI/EXEC is not described as rollback; after dispatch, UI cancellation never
claims server cancellation without proof.

## Phase 9 — daily workflows and data movement

### Deliver

- result-tab pinning and complete multi-operation summaries;
- saved filters, column/object preferences, profile/object/query organization;
- streaming CSV/JSON import/export and engine-appropriate SQL forms;
- type mapping, progress, cancellation cleanup, atomic destination policy, and
  explicit partial import outcomes;
- server/table operations, dashboards, health, reconnect, shutdown, cache/
  eviction, and one-failed-tab isolation;
- support/version matrix and user-facing capability/limitation documentation.

### Exit evidence

Import/export remains bounded and handles malformed data/encoding/formula-like
content safely; incomplete export files are removed; permissions
and unsupported operations are visible; relaunch cannot cause a reconnect storm
or resurrect results/edits.

## Phase 10 — scoped parity expansion

### Deliver

- reviewed structure/DDL editing by engine capability;
- PostgreSQL `pg_dump`/`pg_restore` integration with tool-version, process,
  progress, cancellation, file, and secret controls;
- PostgreSQL relationship exploration and native-diagram-ready graph contract;
- PostgreSQL role/privilege inspection, then only separately reviewed changes;
- reviewed bounded startup SQL/commands;
- optional Vim behavior over the neutral editor state machine;
- applicable maintenance/optimization, explain, and engine-administration rows;
- SSH tunneling through the selected Rust `russh` adapter, with host-key,
  known-hosts, agent/key/password, keepalive, reconnect, and secret tests;
- cloud-provider proxy/identity workflows remain excluded.

### Exit evidence

Every item has real privilege/version/destructive/failure tests and an explicit
unsupported state on other engines. No generic UI invents cross-engine behavior.

## Phase 11 — TUI hardening and parity release gate

### Deliver

- complete unit/model/adapter/integration/render/PTY/real-server suites;
- failure injection for disconnect, timeouts, cancellation races, disk full,
  migration failure, terminal failure, and partial database outcomes;
- measured startup, first-row, resident-scroll, completion, memory, and shutdown
  budgets on the published support matrix;
- terminal accessibility, keyboard/mouse, non-color, Unicode, narrow layout,
  and restoration audits;
- OpenTelemetry only with safe schemas/defaults and an off path;
- clean-room provenance, dependency/license, secret/log, and documentation audit.

### Exit evidence

The TUI satisfies [32-quality-and-verification.md](32-quality-and-verification.md).
Every parity-ledger row is implemented, explicitly excluded by boundary, or
still a visible blocker. Only the first two states permit the corresponding
parity claim.

## Phase 12 — selected native architecture proof

### Deliver

- prove direct Developer ID distribution, hardened runtime, notarization,
  stapling, network/file/Keychain/1Password behavior;
- one stable Rust facade for open/submit/events/page/cancel/shutdown;
- synchronous UniFFI over an embedded Rust static library/XCFramework;
- Swift 6 strict concurrency, `@MainActor` handoff, operation-ID cancellation,
  panic/error mapping, allocation/leak/page/scroll, and universal packaging
  evidence;
- no daemon, local RPC, manual C ABI, or App Store path.

### Exit evidence

The UniFFI bridge and direct distribution pass on clean machines. Failure blocks
native work and requires explicit decision revision. No broad native feature
work starts before cross-adapter conformance passes for all three engines.

## Phase 13 — native vertical slice

### Deliver

- SwiftUI `App`, `WindowGroup`, commands, toolbar, Settings, and restoration;
- `@MainActor` presentation store with no database behavior;
- UniFFI bridge client and immutable event/page decoding;
- connection/profile flow, AppKit catalog, editor, large grid, result page,
  cancel/error/safety review, Keychain adapter, files, and first accessibility
  path;
- bridge/native conformance against the same Rust engine fixtures as TUI.

### Exit evidence

The vertical slice connects, explores, executes, pages, cancels, and shows one
reviewed safe operation on each applicable engine. Swift contains no driver,
parser safety, edit-plan, redaction, or result-authority duplication; there is
no per-cell bridge call.

## Phase 14 — native workflow parity and release evidence

### Deliver

- complete connection organization, workbench tabs, editors, grids, inspectors,
  history/saved/files, edit review, data movement, and engine-specific screens;
- multi-window ownership/restoration, menus/commands, drag/drop/pasteboard,
  security-scoped files, settings, and native appearance;
- VoiceOver, keyboard, focus, selection, marked text/IME, reduced motion,
  contrast, and large-content tests;
- signing, hardened runtime, notarization/stapling,
  credentials, update/migration, crash recovery, and uninstall evidence.

### Exit evidence

Native and TUI produce semantically equivalent Rust outcomes for every shared
workflow. Platform-only behavior remains thin and tested. Clean-machine Release
artifacts pass the selected distribution channel.

## Phase 15 — parity closure and maintenance

### Deliver

- final functional-ledger audit with tests/user docs per row;
- tested server, terminal, macOS, architecture, and migration matrix;
- support diagnostics and redacted failure collection;
- provenance/license/reproducibility/release audit;
- compatibility monitoring for TermRock, Ratatui, database clients, Rust, Swift,
  macOS, servers, 1Password CLI, and selected packaging tools.

### Exit evidence

No in-scope gap is hidden. Release claims list exact engines, versions,
platforms, cancellation limitations, distribution shape, and exclusions.

## Explicitly separate programs

- databases other than PostgreSQL, ClickHouse, and Redis;
- third-party driver/plugin ABI, iOS/iPadOS, team licensing/commerce;
- AI query generation, AI chat, MCP, or external-agent database access;
- any copied reference-product visual language or source-derived implementation.

These require new product/security decisions and do not silently enter a phase.
