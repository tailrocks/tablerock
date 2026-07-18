# Roadmap

TableRock builds one Rust-owned PostgreSQL, ClickHouse, and Redis workbench,
first as a TermRock/Ratatui TUI and later as a native SwiftUI/AppKit macOS
application. Phase 0 decisions were approved on 2026-07-16. The
screen-by-screen product baseline lives in
[docs/product](docs/product/README.md); it was added on 2026-07-18 and is the
authority for what each screen does.

All delivery is direct, forward-only work on `main`. Never create a branch or
pull request. Each checkpoint must build, pass its evidence gate, update its
documentation, and remain honest about incomplete parity.

Completed checkpoints are not narrated here. Each one records an evidence
document; browse them in the [evidence index](docs/evidence/README.md).

## Program map

| Phase | Outcome | Depends on |
|---|---|---|
| 0 | Research and architecture decisions approved | — |
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
| 12 | Selected macOS distribution and UniFFI proof | 11 |
| 13 | Native macOS vertical slice | 12 |
| 14 | Native workflow parity and release evidence | 13 |
| 15 | Parity closure and ongoing compatibility | 14 |

## Phase 0 — approve research

**Status: complete.** Exit evidence:
[34](docs/evidence/phase-0/34-phase-0-exit-report.md).

Freeze the product boundary, clean-room process, functional-parity ledger,
TermRock ownership, TEA, Rust contract language, SecretSource model, local-only
Turso persistence, server support policy, synchronous UniFFI bridge, and direct
notarized macOS distribution. No application code or dependency is added before
this phase is approved.

## Phase 1 — TermRock and TUI foundation

**Status: complete.** Requirement-by-requirement audit:
[45](docs/evidence/phase-1/45-phase-1-exit-report.md); per-checkpoint evidence:
[phase-1](docs/evidence/README.md#phase-1--termrock-substrate-and-tui-shell).

Pin an exact TermRock revision and Ratatui compatibility tuple. Build the sole
TEA Model/Message/Update/Effect/Subscription/View shell, terminal lifecycle,
focus, responsive layout, render harness, and safe shutdown. Add missing neutral
`Form`, `Tree`, `SplitPane`, and interaction primitives to TermRock `main`
first, with reusable APIs, lookbook cases, docs, tests, and Jackin
compatibility. Use Crossterm 0.29 as the sole terminal backend: one CLI
EventStream for input and TermRock's Crossterm session as the sole terminal
lifecycle owner.

## Phase 2 — Rust service foundation

**Status: in progress.** Evidence so far:
[core contracts and services](docs/evidence/README.md#phase-2--core-contracts-and-services),
[profiles](docs/evidence/README.md#phase-2--profiles),
[persistence](docs/evidence/README.md#phase-2--persistence),
[PostgreSQL](docs/evidence/README.md#phase-2--postgresql-driver),
[Redis](docs/evidence/README.md#phase-2--redis-driver),
[ClickHouse](docs/evidence/README.md#phase-2--clickhouse-driver).

Define owned IDs, capabilities, values, revisions, commands, events, pages,
errors, cancellation, safety, and redaction. Implement local-only Turso through
the `turso` crate on one serialized Rust async persistence actor. Run
real-server spikes for `tokio-postgres`, official `clickhouse-rs`, and
`redis-rs`, proving arbitrary values, bounded streaming, TLS, cancellation
truth, reconnect, and ambiguous-write behavior before feature claims.

Done so far:

- Core contracts: identity/revision, bounded owned values, per-engine
  capabilities, immutable columnar pages, operation lifecycle, safe
  diagnostics, typed command envelopes, secret sources, and structured and
  temporal value kinds.
- Application services: operation coordinator, event queue, subscription
  fan-out, object-safe driver adapter, owned driver routing, engine service
  bridge, graceful shutdown, and a three-engine harness proving simultaneous
  bounded execution and per-engine service cancellation.
- Profiles and persistence: versioned property policy, connect-ready
  snapshots, the durable aggregate, bounded CRUD with revision CAS, search and
  filters, and the Turso actor with migrations, crash recovery, and verified
  backup/restore.
- PostgreSQL: bounded typed streaming on 17.10/18.4, custom-root TLS and mTLS,
  cancellation truth including completion races and transport loss, typed
  parameters, notices, multi-statement outcomes, bounded COPY IN/OUT,
  ambiguous write/commit truth, and bounded decoders/projections for the
  scalar, JSON, array, range, composite, domain, enum, network, bit-string,
  identifier, LSN, TID, OID-vector, and snapshot families.
- Redis: binary-safe SCAN and collection scans under concurrent mutation,
  pipelines with per-command partial failure, TTL truth and reviewed TTL
  mutation, timeout/reconnect, TLS/mTLS/ACL, isolated Pub/Sub with pattern
  subscriptions, reconnect and credential-revocation behavior on 7.4.9/8.8.0.
- ClickHouse: RowBinary streaming, complex scalars, structured containers, and
  temporal projection on 25.8/26.3 LTS.
- Performance: current-line 10,000-row streaming, first-page, throughput,
  page-residency, and process-RSS budgets.

Still open: product file effects and UI/UniFFI integration beyond the Phase 3
connection shell. Engine sessions are reusable with operator SQL and health
(193–195); lazy catalog listing works for PG/CH/Redis (198).

## Phase 3 — connection experience

**Status: complete.** Exit evidence: 199–206 (effect bridge, Test/Connect,
list/search/Open, Form/Tree, password prompt + reconnect policy,
describe_server real matrix). Residual deferred by product: URL import,
1Password/Keychain/env sources, group rename dialog polish, delayed
reconnect auto re-dispatch. Temporary Connect, save, Test without save,
prompt-on-connect fail-closed, Remove confirm, and TermRock Form/Tree are
proven on trunk.

## Phase 4 — PostgreSQL read-only tracer

**Status: complete.** Exit evidence: 208–214 (workbench frame/catalog/tabs,
DataGridModel + VirtualGrid, browse first page, SQL/cancel/inspector,
FetchPage pump-and-store multi-page). Residual deferred by product/plan:

- Structure/raw DDL tab (plan 013 unless trivial)
- Full EngineService event-pump cancel race labels on the TUI path (engine
  races proven; UI shows cancel-requested vs cancelled + observed label)
- Server sort/filter/columns (Phase 5 / plan 012)

Catalog, context bar, object preview tabs, bounded 500-row pages with
ResultStore pin, arbitrary SQL streaming, typed display, first-rows-before-
completion, resident-scroll no-I/O, 10k cap, inspector, and cancel dispatch
vs outcome are proven on trunk.

## Phase 5 — workbench foundation

**Status: partial.** Plans 011–012 on trunk (evidence 217–226): TermRock
`TextArea`/`CompletionMenu`, multiline SQL editor, dialect statement
boundaries, revisioned completion, history, saved queries, atomic `.sql`,
intent restore, typed browse-plan sort/filter, column layout persistence,
six copy formats + OSC 52. Still open for full Phase 5 exit: parameters,
find/replace, formatting, explain, VirtualGrid header geometry polish,
Redis command editor mirror.

## Phase 6 — PostgreSQL write/admin slice

Deliver proven editability, typed value editors, inserts/updates/deletes
staged in memory with visible row/cell highlighting, undo/discard, the review
dialog with exact parameterized SQL preview, transactional apply, conflict and
generated-value handling, foreign-key navigation, reviewed table operations,
activity/dashboard, and PostgreSQL-specific structure facts.

## Phase 7 — ClickHouse slice

Deliver databases/objects/DDL, arbitrary dynamic query results through the
official client, complex values, progress/query IDs, honest cancellation,
batch inserts, parts, explain variants, and asynchronous mutation visibility.
Never present ClickHouse mutations as transactions.

## Phase 8 — Redis slice

Deliver logical database isolation, SCAN navigation, namespaces, byte-safe
keys/values, type views, TTL, bounded server overview, command
editor/completion, pipelines, guarded type-specific edits, and honest
post-dispatch cancellation. Automatic browsing never uses `KEYS`.

## Phase 9 — daily workflows and data movement

Complete result tabs, multi-statement outcomes, saved filters/preferences,
streaming CSV/JSON/SQL import/export where meaningful, cancellation cleanup,
table operations, health/activity, robust history/search, file change
handling, restoration, cache/eviction, and cross-engine support documentation.

## Phase 10 — scoped parity expansion

Close planned later rows from the parity ledger: reviewed structure editing,
PostgreSQL backup/restore, relationship exploration, role/privilege inspection,
startup actions, optional Vim behavior, and engine-specific administration.
SSH uses one Rust `russh` tunnel adapter below the database clients. Cloud
provider proxy/identity workflows remain explicitly excluded from this program.
Features that do not apply to an engine render an explicit unsupported
capability.

## Phase 11 — TUI parity release gate

Pass reducer, widget, full-screen, PTY, real-server, failure-injection,
security, accessibility, Unicode, performance, memory, clean-room, license, and
support-matrix gates. Every in-scope parity-ledger row is implemented,
explicitly excluded, or visibly blocks the parity claim.

## Phase 12 — prove the selected native architecture

Prove the direct Developer ID/hardened-runtime/notarized release, Keychain and
1Password behavior, embedded synchronous UniFFI bridge, Swift 6 concurrency,
XCFramework packaging, cancellation, ownership, and performance. Failure blocks
native work and requires an explicit architecture revision; no secondary bridge
or distribution path is carried in the roadmap.

## Phase 13 — native vertical slice

Build the SwiftUI `App`/window/commands/settings shell in the Liquid Glass
design language (see [native macOS
experience](docs/product/native-macos.md)), `@MainActor` presentation store,
UniFFI Rust bridge, connection experience, AppKit catalog, query editor, large
grid, result page, cancellation, and accessibility tracer. Swift contains no
database or safety behavior.

## Phase 14 — native parity and release evidence

Project all supported profiles, tabs, history, query/edit/review,
engine-specific views, import/export, files, settings, restoration,
multi-window behavior, VoiceOver, keyboard, appearance, IME, signing, hardened
runtime/notarization, upgrade, uninstall, crash recovery, and performance
through the shared Rust contracts.

## Phase 15 — close and maintain parity

Audit the functional ledger, user documentation, tested server/terminal/macOS
matrix, provenance, licenses, migrations, support diagnostics, and release
artifacts. Continue compatibility work through small buildable `main` commits;
never hide an unsupported or regressed capability behind a parity claim.

---

Detailed deliverables and phase gates are in the
[delivery plan](docs/architecture/delivery-plan.md). The feature baseline is
the [functional parity ledger](docs/architecture/functional-parity-ledger.md).
All phases obey the forward-only dependency policy and automated freshness gate
in the [dependency policy](docs/architecture/dependency-policy.md).
