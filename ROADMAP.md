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

**Mostly complete (evidence 227–234, 323–327):** write path, editability,
staged drafts, typed review, RETURNING, consume-once registry + re-review
on expiry, FK/structure with index + constraint defs, multi-column FK
follow, gated truncate/drop/rename, activity + cancel/terminate with
permission-denied signals, mutation apply Unknown on interrupted COMMIT,
temporal/structured staging validation. Residual polish: optional
calendar/JSON tree widgets.

Deliver proven editability, typed value editors, inserts/updates/deletes
staged in memory with visible row/cell highlighting, undo/discard, the review
dialog with exact parameterized SQL preview, transactional apply, conflict and
generated-value handling, foreign-key navigation, reviewed table operations,
activity/dashboard, and PostgreSQL-specific structure facts.

## Phase 7 — ClickHouse slice

**Mostly complete (evidence 236–239, 257, 305, 308, 320–321):** structure
facts, progressive INSERT, async UPDATE/DELETE + `system.mutations` poll, KILL
MUTATION destructive gate, multi-engine ExecuteSql (CH query_id + summary
progress on status), four-state cancel UI, EXPLAIN raw/AST + editor tree.
Custom CA/mTLS upstream-blocked on clickhouse-rs 0.15.1 (private HttpClient).

Deliver databases/objects/DDL, arbitrary dynamic query results through the
official client, complex values, progress/query IDs, honest cancellation,
batch inserts, parts, explain variants, and asynchronous mutation visibility.
Never present ClickHouse mutations as transactions.

## Phase 8 — Redis slice

**Partial→mostly complete (evidence 241–243):** namespace projection, SCAN
keys action, type/list/stream views, INFO snapshot, command
tokenizer/classifier, sequential SET/DEL/HSET/HDEL/SADD/SREM/ZADD/ZREM
apply, KEYS ban, SCAN MATCH + HSCAN/SSCAN/ZSCAN first-page key views,
key-view stage RAdd/RRem → review/apply, RMore collection pages, command
editor sequential pipeline with per-command ok/err, curated command
completion, isolated BLPOP/BRPOP disposable connection, Pub/Sub Sub/PSub
with multi-page pump + idle stop (evidence 311–317, 322, 329–330). Residual
polish: listen-until-Cancel without idle stop (optional).

Deliver logical database isolation, SCAN navigation, namespaces, byte-safe
keys/values, type views, TTL, bounded server overview, command
editor/completion, pipelines, guarded type-specific edits, and honest
post-dispatch cancellation. Automatic browsing never uses `KEYS`.

## Phase 9 — daily workflows and data movement

**Mostly complete (evidence 245–246, 306–307, 310, 318–319):** atomic file
export, loaded-result CSV/JSON/TSV export, CSV import parse (formula-neutral),
multi-statement result sections + explicit RunScript full-buffer path, saved
filter JSON library + Turso actor persist (schema 13) + named SaveFilt/LoadFilt
+ connect-path load + fuzzy unique-match apply, manual reconnect policy.
Residual polish only if product asks for full list-navigation UI.

Complete result tabs, multi-statement outcomes, saved filters/preferences,
streaming CSV/JSON/SQL import/export where meaningful, cancellation cleanup,
table operations, health/activity, robust history/search, file change
handling, restoration, cache/eviction, and cross-engine support documentation.

## Phase 10 — scoped parity expansion

**Mostly complete (plan 017, evidence 260–284):** SSH russh bastion matrix
(password/pubkey/encrypted/agent, known_hosts fail-closed, multi-engine
forward), keepalive defaults, pg_dump/pg_restore supervision + Docker matrix,
reviewed DDL + TUI, roles/effective membership + inspector, startup actions
(ReadOnly auto + Write/Danger review), Vim keymap layer, relationship graph
contract. Residual polish: CI client packages for non-skip dump runs.

Cloud-provider proxy/identity remains excluded. Features that do not apply to
an engine render an explicit unsupported capability.

## Phase 11 — TUI parity release gate

**Mostly complete (plan 018, evidence 248–303, 309, 328):** redaction,
non-color cues, OTLP-off, export fail-closed, resize storm, disconnect
mid-stream marks live ops, ledger three-state CSV, local and ubuntu CI
first-row budgets, URL import/external open, explain tree, named params,
multi-statement script selection, find/replace, format SQL, ENOSPC 1MiB
tmpfs CI. Residual polish: fixed-spec multi-runner first-paint numbers.

Every in-scope parity-ledger row is implemented, explicitly excluded, or
visibly blocks (Native multi-window waits on Phase 12 packaging).

## Phase 12 — prove the selected native architecture

Prove the direct Developer ID/hardened-runtime/notarized release, Keychain and
1Password behavior, embedded synchronous UniFFI bridge, Swift 6 concurrency,
XCFramework packaging, cancellation, ownership, and performance. Failure blocks
native work and requires an explicit architecture revision; no secondary bridge
or distribution path is carried in the roadmap.

**Software gate complete (plan 019):** page v1 codec, `tablerock-ffi` facade,
UniFFI Swift bindings, conformance (stubs + Docker PG/CH/Redis), review/apply
handles, profile open, universal lipo staticlib, Swift `PageV1` decode, and CLT
proof harness are on `main`. **Distribution gate operator-blocked:** full Xcode
(XCFramework) + Developer ID notarize/staple/clean-machine (see
`docs/evidence/delivery/251-plan-019-operator-stop.md`). Phase 13 waits on that
distribution proof per plan 020 entry gate.

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
