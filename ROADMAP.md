# Roadmap

TableRock completed Phase 0 decision approval on 2026-07-16. The program
builds one Rust-owned PostgreSQL, ClickHouse, and Redis workbench, first as a
TermRock/Ratatui TUI and later as a native SwiftUI/AppKit macOS application.

All delivery is direct, forward-only work on `main`. Never create a branch or
pull request. Each checkpoint must build, pass its evidence gate, update its
documentation, and remain honest about incomplete parity.

## Program map

| Phase | Outcome | Depends on |
|---|---|---|
| 0 | Research and architecture decisions approved (complete) | current research |
| 1 | TermRock substrate and empty TUI shell | 0 |
| 2 | Rust contracts, storage choice, and three driver spikes | 0 |
| 3 | Profiles, credentials, and connection shell | 1, 2 |
| 4 | PostgreSQL read-only vertical slice | 3, TermRock tree/grid |
| 5 | Editor, completion, grid, history, and saved-query foundation | 4, TermRock editor primitives |
| 6 | PostgreSQL editing and administration | 5 |
| 7 | ClickHouse complete engine slice | 5 |
| 8 | Redis complete engine slice | 5 |
| 9 | Cross-engine data movement and daily workflows | 6, 7, 8 |
| 10 | Scoped parity expansion | 9 |
| 11 | TUI hardening and parity release gate | 10 |
| 12 | selected macOS distribution and UniFFI proof | 11 |
| 13 | Native macOS vertical slice | 12 |
| 14 | Native workflow parity and release evidence | 13 |
| 15 | Parity closure and ongoing compatibility | 14 |

## Phase outcomes

### Phase 0 — approve research

**Status:** Complete. The operator approved all fixed decisions and authorized
Phases 0-15 on 2026-07-16. Exit evidence is recorded in
[`34-phase-0-exit-report.md`](docs/research/34-phase-0-exit-report.md).

Freeze the product boundary, clean-room process, functional-parity ledger,
TermRock ownership, TEA, Rust contract language, SecretSource model, local-only
Turso persistence, server support policy, synchronous UniFFI bridge, and direct
notarized macOS distribution. No application code or dependency is added before
this phase is approved.

### Phase 1 — TermRock and TUI foundation

**Status:** Complete. The requirement-by-requirement audit is recorded in
[`45-phase-1-exit-report.md`](docs/research/45-phase-1-exit-report.md). T0 pins and verifies the minimal TermRock consumer in
[`35-phase-1-termrock-t0.md`](docs/research/35-phase-1-termrock-t0.md). The T1
`Tree`, `Form`, and `SplitPane` checkpoints are published and repinned with evidence in
[`36-phase-1-termrock-tree.md`](docs/research/36-phase-1-termrock-tree.md) and
[`37-phase-1-termrock-form.md`](docs/research/37-phase-1-termrock-form.md), and
[`38-phase-1-termrock-split-pane.md`](docs/research/38-phase-1-termrock-split-pane.md).
Full-screen lifecycle implementation and compatibility evidence are repinned in
[`41-phase-1-terminal-lifecycle.md`](docs/research/41-phase-1-terminal-lifecycle.md).
The root TEA module boundaries, deterministic reducer, bounded subscription
declarations, responsive shell projection, focus order, minimum-size state, and
`TestBackend` evidence are implemented. The executable owns one EventStream,
maps backend input into semantic messages, renders only dirty frames, contains
panics, handles Ctrl-C/SIGTERM, rejects non-TTY execution, and has real-PTY
normal/signal restoration evidence. It instantiates the declared bounded
post-mapping root queue. Render-authorized mouse/paste/focus routing is recorded
in [`42-phase-1-render-authorized-input.md`](docs/research/42-phase-1-render-authorized-input.md).
Returned-error and panic PTY restoration evidence is recorded in
[`43-phase-1-fault-restoration.md`](docs/research/43-phase-1-fault-restoration.md).
Generic post-mapping progress coalescing and explicit overflow/resync evidence
is recorded in
[`44-phase-1-bounded-ingress.md`](docs/research/44-phase-1-bounded-ingress.md).
Domain event identity/revision mapping belongs to Phase 2.
The historical TermRock 0.8 migration is recorded in
[`49-termrock-0.8-migration.md`](docs/research/49-termrock-0.8-migration.md); the
latest 0.9 migration and exact current-main pin are recorded in
[`57-termrock-0.9-migration.md`](docs/research/57-termrock-0.9-migration.md).
The typed OSC and unknown-key migrations, with the refreshed exact main pin,
are recorded in
[`59-termrock-0.9-input-osc-migration.md`](docs/research/59-termrock-0.9-input-osc-migration.md).
TermRock's subsequent unified key-vocabulary migration and refreshed exact
main pin are recorded in
[`61-termrock-0.9-key-vocabulary-migration.md`](docs/research/61-termrock-0.9-key-vocabulary-migration.md).
The local-only Turso adoption, single-owner serialized actor, sequential
migrations, and initial real-file compatibility evidence are recorded in
[`58-phase-2-persistence-actor-foundation.md`](docs/research/58-phase-2-persistence-actor-foundation.md).
Normalized single-actor ownership and transactional interrupted-migration
recovery rules are recorded in
[`60-phase-2-persistence-ownership-recovery.md`](docs/research/60-phase-2-persistence-ownership-recovery.md).
Abrupt subprocess death without destructor/checkpoint and verified reopen are
recorded in
[`62-phase-2-persistence-crash-recovery.md`](docs/research/62-phase-2-persistence-crash-recovery.md).
Sequential saved-profile schema migration `0003` and the saved-token-only
atomic create tracer are recorded in
[`63-phase-2-saved-profile-create.md`](docs/research/63-phase-2-saved-profile-create.md).
TermRock's semantic constructible-theme migration and exact refreshed main pin
are recorded in
[`64-termrock-0.9-constructible-theme-migration.md`](docs/research/64-termrock-0.9-constructible-theme-migration.md).
Strict transactional saved-profile decoding, not-found semantics, redaction,
and reopen evidence are recorded in
[`65-phase-2-saved-profile-read.md`](docs/research/65-phase-2-saved-profile-read.md).
Atomic saved-profile replacement with exact revision compare-and-swap and
rollback evidence is recorded in
[`66-phase-2-saved-profile-replace.md`](docs/research/66-phase-2-saved-profile-replace.md).
TermRock's semantic-palette cleanup and exact refreshed main pin are recorded in
[`67-termrock-0.9-semantic-palette-migration.md`](docs/research/67-termrock-0.9-semantic-palette-migration.md).
Revision-CAS deletion and profile-owned child cleanup evidence are recorded in
[`68-phase-2-saved-profile-delete.md`](docs/research/68-phase-2-saved-profile-delete.md).

Pin an exact TermRock revision and Ratatui compatibility tuple. Build the sole
TEA Model/Message/Update/Effect/Subscription/View shell, terminal lifecycle, focus,
responsive layout, render harness, and safe shutdown. Add missing neutral
`Form`, `Tree`, `SplitPane`, and interaction primitives to TermRock `main` first,
with reusable APIs, lookbook cases, docs, tests, and Jackin compatibility.
Use Crossterm 0.29 as the sole terminal backend: one CLI EventStream for input
and TermRock's Crossterm session as the sole terminal lifecycle owner.

### Phase 2 — Rust service foundation

**Status:** In progress. The dependency-minimal authoritative ID and monotonic revision
tracer is recorded in
[`46-phase-2-core-identity.md`](docs/research/46-phase-2-core-identity.md).
The bounded owned-value and explicit per-engine capability tracer is recorded in
[`47-phase-2-value-capability-contract.md`](docs/research/47-phase-2-value-capability-contract.md).
The pre-allocation-bounded immutable columnar page tracer is recorded in
[`48-phase-2-page-contract.md`](docs/research/48-phase-2-page-contract.md).
The live-session operation lifecycle and event-identity tracer is recorded in
[`50-phase-2-operation-lifecycle.md`](docs/research/50-phase-2-operation-lifecycle.md).
The redacted failure, ambiguity, and retry-policy tracer is recorded in
[`51-phase-2-safe-diagnostics.md`](docs/research/51-phase-2-safe-diagnostics.md).
The typed scope, finite budget, and versioned command-envelope tracer is
recorded in
[`52-phase-2-command-envelope.md`](docs/research/52-phase-2-command-envelope.md).
The versioned, redacted secret-source reference tracer is recorded in
[`53-phase-2-secret-source.md`](docs/research/53-phase-2-secret-source.md).
The versioned profile property/source policy that forbids ordinary literal
secret material is recorded in
[`54-phase-2-profile-property-policy.md`](docs/research/54-phase-2-profile-property-policy.md).
The immutable connect-ready profile connection snapshot, TLS state machine, two-mode
safety policy, and finite owner limits are recorded in
[`55-phase-2-profile-snapshot.md`](docs/research/55-phase-2-profile-snapshot.md).
The baseline durable profile aggregate, saved/temporary disposition, bounded
organization/preferences, and revision replacement gate are recorded in
[`56-phase-2-profile-aggregate.md`](docs/research/56-phase-2-profile-aggregate.md).

Define owned IDs, capabilities, values, revisions, commands, events, pages,
errors, cancellation, safety, and redaction. Implement local-only Turso through
the `turso` crate on one serialized Rust async persistence actor.
Run real-server spikes for `tokio-postgres`, official `clickhouse-rs`, and
`redis-rs`, proving arbitrary values, bounded streaming, TLS, cancellation
truth, reconnect, and ambiguous-write behavior before feature claims.

### Phase 3 — connection experience

Deliver searchable/organized profiles, capability-driven forms, URL and
temporary drafts, Test and Connect, General/TLS/Safety sections, 1Password
references, explicit plaintext danger, context selection, health, reconnect,
and safe versioned persistence for all three engines.

### Phase 4 — PostgreSQL read-only tracer

Deliver lazy database/schema/object catalog, object and query tabs, structure,
bounded table pages, arbitrary SQL streaming, typed values, progress, cancel,
errors, and result inspection. Add TermRock `VirtualGrid` before TableRock ships
its database-aware grid composition.

### Phase 5 — workbench foundation

Add TermRock `TextArea` and `CompletionMenu`; then deliver multiline SQL/Redis
editing, statement selection, revisioned completion, parameters, find/replace,
formatting, explain foundations, sorting/filtering, column controls, copy
formats, query files, query history, favorites, saved queries, quick switching,
and intent-only session restoration.

### Phase 6 — PostgreSQL write/admin slice

Deliver proven editability, typed value editors, inserts/updates/deletes,
staged review/undo/discard, parameterized transactional apply, conflict and
generated-value handling, foreign-key navigation, reviewed table operations,
activity/dashboard, and PostgreSQL-specific structure facts.

### Phase 7 — ClickHouse slice

Deliver databases/objects/DDL, arbitrary dynamic query results through the
official client, complex values, progress/query IDs, honest cancellation,
batch inserts, parts, explain variants, and asynchronous mutation visibility.
Never present ClickHouse mutations as transactions.

### Phase 8 — Redis slice

Deliver logical database isolation, SCAN navigation, namespaces, byte-safe
keys/values, type views, TTL, bounded server overview, command editor/completion,
pipelines, guarded type-specific edits, and honest post-dispatch cancellation.
Automatic browsing never uses `KEYS`.

### Phase 9 — daily workflows and data movement

Complete result tabs, multi-statement outcomes, saved filters/preferences,
streaming CSV/JSON/SQL import/export where meaningful, cancellation cleanup,
table operations, health/activity, robust history/search, file change handling,
restoration, cache/eviction, and cross-engine support documentation.

### Phase 10 — scoped parity expansion

Close planned later rows from the parity ledger: reviewed structure editing,
PostgreSQL backup/restore, relationship exploration, role/privilege inspection,
startup actions, optional Vim behavior, and engine-specific administration.
SSH uses one Rust `russh` tunnel adapter below the database clients. Cloud
provider proxy/identity workflows remain explicitly excluded from this program.
Features that do not apply to an engine render an explicit unsupported
capability.

### Phase 11 — TUI parity release gate

Pass reducer, widget, full-screen, PTY, real-server, failure-injection,
security, accessibility, Unicode, performance, memory, clean-room, license, and
support-matrix gates. Every in-scope parity-ledger row is implemented, explicitly
excluded, or visibly blocks the parity claim.

### Phase 12 — prove the selected native architecture

Prove the direct Developer ID/hardened-runtime/notarized release, Keychain and
1Password behavior, embedded synchronous UniFFI bridge, Swift 6 concurrency,
XCFramework packaging, cancellation, ownership, and performance. Failure blocks
native work and requires an explicit architecture revision; no secondary bridge
or distribution path is carried in the roadmap.

### Phase 13 — native vertical slice

Build the SwiftUI `App`/window/commands/settings shell, `@MainActor`
presentation store, UniFFI Rust bridge, connection experience, AppKit catalog,
query editor, large grid, result page, cancellation, and accessibility tracer.
Swift contains no database or safety behavior.

### Phase 14 — native parity and release evidence

Project all supported profiles, tabs, history, query/edit/review, engine-specific
views, import/export, files, settings, restoration, multi-window behavior,
VoiceOver, keyboard, appearance, IME, signing, hardened runtime/notarization, upgrade,
uninstall, crash recovery, and performance through the shared Rust contracts.

### Phase 15 — close and maintain parity

Audit the functional ledger, user documentation, tested server/terminal/macOS
matrix, provenance, licenses, migrations, support diagnostics, and release
artifacts. Continue compatibility work through small buildable `main` commits;
never hide an unsupported or regressed capability behind a parity claim.

Detailed deliverables and phase gates are in
[the delivery plan](docs/research/30-delivery-plan.md). The feature baseline is
[the functional parity ledger](docs/research/06-functional-parity-ledger.md).
