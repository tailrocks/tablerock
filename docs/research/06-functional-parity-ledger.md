# Functional Parity Ledger

## Decision

TableRock targets a complete PostgreSQL, ClickHouse, and Redis workbench with
terminal and native macOS clients. TablePro is evidence that broad workflows
exist, not a visual or implementation specification.

“Parity” means an operator can accomplish the same relevant database task with
equivalent safety, state visibility, and failure handling. It does **not** mean
the same layout, words, geometry, colors, icons, shortcuts, assets, or internal
architecture. TableRock requirements come from this repository, official
database/client/platform documentation, and direct tests.

This ledger was refreshed on 2026-07-16 from TablePro's public
[feature index](https://docs.tablepro.app/features/overview),
[connection guide](https://docs.tablepro.app/databases/overview),
[SQL editor guide](https://docs.tablepro.app/features/sql-editor),
[data-grid guide](https://docs.tablepro.app/features/data-grid),
[safety guide](https://docs.tablepro.app/features/safe-mode), and
[import/export guide](https://docs.tablepro.app/features/import-export). No
TablePro source, tests, identifiers, product text, asset, screenshot, or layout
measurement is an implementation input.

## Status vocabulary

| Status | Meaning |
|---|---|
| Core | Required before the terminal product is considered complete |
| Parity | Required before the three-engine workbench claims functional parity |
| Native | Required for the native macOS client, after the Rust service contract stabilizes |
| Later | Valid product work, deliberately sequenced after core parity |
| Excluded | Outside TableRock's product boundary unless a new decision changes it |

The status is sequencing, not permission to ship known-wrong behavior. A
deferred item remains an explicit gap and is never presented as implemented.

## Product boundary

The parity claim covers only:

- PostgreSQL;
- ClickHouse;
- Redis;
- terminal TUI on supported terminal platforms;
- native macOS UI over the same Rust-owned behavior.

The claim excludes TablePro's other database drivers, plugin ecosystem, iOS
client, licensing/team commerce, AI assistant, MCP server, and vendor-specific
sync behavior. Those are different products or require separate security and
architecture decisions.

## Connection and launch workflows

| Capability | Status | TableRock requirement | Acceptance evidence |
|---|---|---|---|
| Connection list | Core | Searchable profiles with engine, endpoint, safety, secret source, health, and explicit empty/loading/failure states | Keyboard and mouse conformance fixtures at narrow/normal/wide sizes |
| Create/edit/duplicate/remove | Core | One capability-driven form for the three engines; removing a profile never silently removes unrelated history or active work | Pure reducer tests plus persistence migration tests |
| Engine chooser | Core | Exactly PostgreSQL, ClickHouse, and Redis; no dormant plugin affordance | Snapshot and schema tests reject unknown engines |
| URL import | Parity | Parse supported database URLs into a reviewable draft; credentials remain transient unless the operator selects a secret destination | Parser fixtures cover percent encoding, TLS, missing fields, and redaction |
| External URL open | Later | Confirm target and safety before opening a temporary or matching saved session | Deep-link threat model and hostile-input tests |
| Test connection | Core | Show server identity/version, TLS outcome, elapsed time, and redacted diagnostics without saving | Real TLS/auth fixtures for all engines |
| Temporary connection | Core | Connect without persisting profile or secret | Relaunch proves no durable profile/secret remains |
| Profile organization | Parity | Groups, tags, favorites, ordering, filters, and environment/safety markers without copied reference presentation | Phase 2 proves profile contracts plus a local-only serialized Turso actor, normalized process-local single ownership, interrupted/crash recovery, sequential profile migrations, atomic CRUD, and a least-data keyset-paginated organization list capped at 100 with cursor-bound filters, bounded Unicode search, and validated literal/unresolved-secret endpoint facts; live health projection, resolution, UI, unrelated-entity retention, and remaining storage proof stay required |
| Context switcher | Core | Switch connection/database/schema/logical database using engine-correct semantics | Driver contract tests prove no fake cross-engine abstraction |
| Health and reconnect | Core | Visible state, bounded backoff, authentication stop condition, context restoration, and no automatic ambiguous-write retry | Disconnect/reconnect race harness |
| Startup actions | Later | Reviewed, bounded startup SQL/commands with explicit reconnect behavior | Safety classification, timeout, and partial-failure tests |
| SSH transport | Parity | Rust `russh` adapter below clients with host-key verification, known-hosts, key/agent/password auth, keepalive, cancel, and reconnect | Real SSH bastion matrix; no shell interpolation or secret logging |
| Cloud-provider proxy/identity | Excluded | No vendor proxy/identity workflow in this three-engine program | Product-boundary test and documentation |

## Workbench and navigation

| Capability | Status | TableRock requirement | Acceptance evidence |
|---|---|---|---|
| Lazy catalog | Core | Engine-native hierarchy, loading/stale/error nodes, subtree refresh, filtering with ancestors preserved | Phase 2 now proves bounded immutable engine-native preorder snapshots, stable IDs/parents, lazy/loading/stale/partial/failed child states, safe failure diagnostics, 10,001-node synthetic scale, hostile hierarchy/type/text/depth rejection, and stale/gap revision rejection; driver subtree refresh, ancestor-preserving filtering, and UI projection remain required |
| Object tabs | Core | Preview/pinned/durable object tabs with independent context and state | Restore and close-policy tests |
| Query/command tabs | Core | Independent text, cursor, context, results, errors, history, and running operation | Phase 2 now proves typed command scopes, finite budgets, scoped operation identity, legal lifecycle/cancellation-outcome edges, stale/duplicate rejection, gap resync, one core-authoritative application-service harness with simultaneous bounded PostgreSQL, ClickHouse, and Redis tasks, three-engine cancellation truth, and initial current-line streaming budgets; execution text, multi-tab behavior, release-profile budgets, and remaining server races are required |
| Result tabs | Parity | One result per statement/operation, pinning, completion summaries, failure and partial-result states | Phase 2 now proves a message-free safe diagnostic taxonomy with explicit ambiguity/retry facts; multi-statement and real-driver mapping fixtures remain required |
| Responsive layout | Core | Wide split view, medium constrained view, narrow single-region navigation, explicit minimum-size screen | Render fixtures with Unicode and extreme labels |
| Quick switcher | Parity | Fuzzy switch across visible objects, profiles, tabs, and saved queries using stable IDs | Ranking and stale-index tests |
| Favorites and saved queries | Parity | Table/object favorites and named query files with explicit scope | Persistence, rename, and missing-target tests |
| Session restoration | Core | Restore intent and text, never result payloads or pending writes by default | Crash/relaunch and schema-migration tests |
| Multi-window | Native | Independent native windows over shared Rust sessions with explicit ownership | macOS restoration and multi-window UI tests |

## SQL and command editor

| Capability | Status | TableRock requirement | Acceptance evidence |
|---|---|---|---|
| Multiline editing | Core | Unicode-safe buffer, cursor/selection, undo/redo, line numbers, scroll, paste, and search | Buffer property tests and terminal render fixtures |
| Syntax projection | Core | PostgreSQL/ClickHouse SQL and Redis command spans computed outside rendering | Incomplete/invalid document corpus |
| Statement selection | Core | Execute selection or current statement without naive semicolon splitting | Procedures, comments, strings, and multi-statement fixtures |
| Revisioned completion | Core | Keywords, catalogs, aliases, types, functions, and Redis commands; old results cannot apply | Race tests over edits and context changes |
| Query parameters | Parity | Named parameters become prepared/bound values where the engine supports them; never string substitution | Injection and type-conversion tests |
| Find/replace | Parity | Literal, case, word, and regular-expression modes with explicit scope | Unicode and zero-width-match tests |
| Formatting | Parity | Dialect-aware formatting preserves comments, literals, identifier quoting, and cursor intent | Golden corpus owned by TableRock |
| Query limits | Core | Enforce server-side limits only after parser proof; otherwise bound result consumption below presentation | Clause-order and misleading-limit tests |
| Explain | Parity | Raw and structured plans; engine-specific modes stay explicit | Versioned plan parsers with unknown-node fallback |
| Vim mode | Later | Optional, documented modal editing built on a neutral editor contract | Independent keymap and mode-transition suite |
| SQL files | Parity | Open/save/reload/diff external changes; safe file permissions and unsaved-change policy | Atomic save and external-modification tests |
| Query history | Core | Searchable, bounded, local history with configurable SQL-text retention; disabled/private modes available | Retention, redaction, and migration tests |

## Grid, values, and changes

| Capability | Status | TableRock requirement | Acceptance evidence |
|---|---|---|---|
| Virtualized grid | Core | Render only resident rows/columns; stable two-axis navigation and placeholders; no I/O from render | Million-row synthetic viewport benchmark |
| Typed display | Core | Distinguish NULL, empty, whitespace, zero, false, bytes, structured containers, unknown, invalid, and truncated values | The Phase 2 owned-value unit contract covers null/empty/whitespace/zero/false/bytes/structured/temporal/unknown/invalid/truncated distinctions; real ClickHouse 25.8/26.3 LTS fixtures now prove booleans, all scalar integer widths through 256 bits, Decimal256, floats, canonical temporal values, UUID/IP, enums, low-cardinality text, nullable text, binary, recursive arrays/tuples/maps/named nested records, bounded structured projection, and bounded unknown fallback; PostgreSQL/Redis breadth remains required |
| Column controls | Parity | Width, fit, hide, order, format, and stable per-object preferences | Narrow/wide/Unicode geometry tests |
| Sorting | Core | Server sort for table browsing, explicit provenance, multi-column order, no unsafe SQL concatenation | Hostile identifier/type fixtures |
| Filtering | Core | Typed server filters plus clearly labeled resident-page value filters; saved presets later | Parameterization and NULL semantics tests |
| Selection/copy | Parity | Cell/range/row selection and safe TSV/CSV/JSON/Markdown projections | Clipboard-neutral formatter tests |
| Paging | Core | Bounded server pages for objects and bounded streams for arbitrary queries; totals may be estimated/unknown | The Phase 2 immutable-page and result-store contracts cover pre-allocation dimensions, ranges, offsets, null/truncation metadata, hostile buffers, explicit result opening, stale/future/overlap rejection, pinned transactional capacity failure, exact resident buffer accounting, and deterministic global LRU eviction; PostgreSQL 17/18, ClickHouse 25.8/26.3, and Redis 7.4/8.8 Testcontainers prove bounded streams through one object-safe adapter/page-stream seam, while driver/store race integration, bridge encoding, and the complete operation matrix remain required |
| Row/value inspector | Core | Full typed values, raw bytes/hex, JSON text/tree, metadata, and stale state | Large/binary/invalid JSON fixtures |
| Type-specific editors | Parity | Bool, number, temporal, enum, JSON, bytes, array/map/tuple, and explicit unknown fallback | Round-trip and invalid-input tests |
| Stable editability | Core | Only results with proven base object and stable identity are editable | Joins/aggregates/no-key/duplicate-key tests |
| Staged changes | Core | Inserts/updates/deletes remain local, visible, undoable, reviewable, and discardable until apply | Phase 2 now provides bounded typed plans, move-only review/authorization, and a bounded exact-once token registry; reducer undo/discard/quit policy and service wiring remain required |
| Operation preview | Core | Preview is descriptive; execution uses a typed parameter/command plan, never reparsed display text | Phase 2 proves execution retains the exact typed plan; bounded preview formatting and native bridge conformance remain required |
| Conflict handling | Core | PostgreSQL conflicts roll back; ClickHouse and Redis expose their true non-transactional outcomes | Phase 2 distinguishes PostgreSQL atomic transactions, ClickHouse progressive inserts/asynchronous mutations, and Redis sequential no-rollback work; concurrent real-server tests remain required |
| Foreign-key navigation | Parity | PostgreSQL relationship lookup and navigation with explicit unavailable state elsewhere | Catalog and permission fixtures |

## Schema, administration, and data movement

| Capability | Status | TableRock requirement | Acceptance evidence |
|---|---|---|---|
| Structure inspection | Core | Columns, keys/indexes, constraints, engine facts, and DDL/raw metadata | Versioned catalog fixtures |
| Structure editing | Later | Capability-gated reviewed DDL; PostgreSQL first, ClickHouse-specific forms, no Redis fiction | Destructive-operation and rollback/outcome tests |
| Table operations | Parity | Refresh, rename where valid, truncate/drop, maintenance/optimize, and copied DDL behind typed safety gates | Per-engine privilege and destructive tests |
| Import | Parity | Streaming CSV/JSON and reviewed SQL where meaningful; mapping, transaction/outcome policy, progress, cancel | Malformed input, formula, encoding, and partial-failure fixtures |
| Export | Parity | Streaming CSV/JSON plus engine-appropriate SQL; atomic destination and cancellation cleanup | Constant-memory and partial-file tests |
| Backup/restore | Later | PostgreSQL tool integration with version checks, progress, cancel, and secret-safe process invocation | Real `pg_dump`/`pg_restore` matrix |
| ER relationships | Later | PostgreSQL relationship graph; terminal tree/list first, native diagram later | Cycles, large graph, and missing-FK tests |
| Server dashboard | Parity | Current bounded health/activity snapshots, cancel/kill only through explicit capability and confirmation | Permission-denied and version-drift tests |
| Users and roles | Later | PostgreSQL-only role/privilege inspection before reviewed mutation support | Effective-privilege and self-lockout tests |

## Engine-specific parity

Public reference pages establish only the broad workflows. Official engine
documentation defines behavior:

- [TablePro PostgreSQL page](https://docs.tablepro.app/databases/postgresql):
  schemas, native types, structure, role, and backup workflows exist. TableRock
  independently defines these through PostgreSQL catalogs/protocol tests.
- [TablePro ClickHouse page](https://docs.tablepro.app/databases/clickhouse):
  database/table browsing, parts, explain variants, cancellation, inserts, and
  asynchronous mutations exist. TableRock uses the official
  [`ClickHouse/clickhouse-rs`](https://github.com/ClickHouse/clickhouse-rs)
  client and never presents mutations as transactions.
- [TablePro Redis page](https://docs.tablepro.app/databases/redis): key
  namespaces, typed values, TTL, commands, and bounded server context exist.
  TableRock uses [`redis-rs/redis-rs`](https://github.com/redis-rs/redis-rs)
  and official Redis command/SCAN semantics; it never uses `KEYS` for automatic
  browsing or invents stable totals.

Detailed engine contracts remain in
[03-database-capabilities.md](03-database-capabilities.md).

## Native macOS parity

Native parity is behavior parity, not terminal emulation inside a window.

| Concern | Native requirement |
|---|---|
| App lifecycle | SwiftUI `App`, scenes, commands, settings, restoration, and multiple windows |
| Catalog and grid | AppKit outline/table views where measured scale or interaction demands them |
| Editor | Native AppKit text input, selection, IME, find, accessibility, and completion presentation |
| Safety | Rust policy plus native review/authentication UI; Swift cannot bypass policy |
| Files and clipboard | Native panels, security-scoped access where required, pasteboard, drag/drop |
| Accessibility | Complete VoiceOver labels, focus order, keyboard access, reduced-motion/contrast behavior |
| Appearance | Native light/dark/system materials and user preferences, not copied reference colors |
| Performance | Coarse immutable pages and events; never one Rust/IPC call per cell |

The ownership and transport decision is in
[12-native-macos-path.md](12-native-macos-path.md) and
[14-shared-client-contract.md](14-shared-client-contract.md).

## Explicit exclusions

- any database other than PostgreSQL, ClickHouse, and Redis;
- TablePro driver plugins or a third-party driver ABI;
- copied TablePro/TablePlus/Zedis layouts, assets, text, themes, or shortcuts;
- iOS/iPadOS;
- licensing, seats, or team commerce;
- AI chat, AI query generation, and MCP/external-agent database access in the
  parity program;
- cloud-provider proxy and identity integrations;
- iCloud as the authoritative state store;
- presenting a desktop screenshot inside the TUI.

## Closure rule

Every row must end in one of three states before a parity claim:

1. implemented with linked tests and user documentation;
2. explicitly excluded by a fixed product-boundary decision; or
3. still listed as a visible gap, which blocks the corresponding parity claim.

The ledger is reviewed at every roadmap phase exit. A new public reference
feature is not automatically adopted; it is evaluated against the three-engine
product boundary, clean-room rule, safety architecture, and official contracts.

## Roadmap checkpoint reviews

| Phase | Evidence | Ledger result |
|---|---|---|
| Phase 1 — TermRock and TUI foundation | [`45-phase-1-exit-report.md`](45-phase-1-exit-report.md) | Reviewed 2026-07-16. The empty shell closes no Core, Parity, or Later product capability. All unimplemented rows remain visible blockers for their owning parity claims; fixed exclusions are unchanged. |
| Phase 2 — core identity/revision tracer | [`46-phase-2-core-identity.md`](46-phase-2-core-identity.md) | Reviewed 2026-07-16. Canonical identity and stale/gap classification infrastructure closes no user capability. Identity namespace provisioning and every remaining Phase 2 contract/storage/driver row stay visible blockers. |
| Phase 2 — persistence backup/restore | [`135-phase-2-persistence-backup-restore.md`](135-phase-2-persistence-backup-restore.md) | Reviewed 2026-07-17. The bounded offline state-store backup and restore primitive closes no PostgreSQL tool-level backup/restore capability. Phase 10 `pg_dump`/`pg_restore`, destructive operator replacement UX, and remaining Phase 2 storage fault/deployment gates stay visible blockers. |
| Phase 2 — PostgreSQL TLS/client identity | [`136-phase-2-postgresql-tls-identity.md`](136-phase-2-postgresql-tls-identity.md) | Reviewed 2026-07-17. Verified custom-root TLS and mTLS pass on PostgreSQL 17.10/18.4, but the cross-engine Test Connection capability remains open until all engines, resolved secret sourcing, elapsed/server identity facts, and presentation flows pass. |
| Phase 2 — Redis pipeline partial failures | [`138-phase-2-redis-pipeline-partial-failure.md`](138-phase-2-redis-pipeline-partial-failure.md) | Reviewed 2026-07-17. Redis 7.4.9/8.8.0 under RESP2/RESP3 prove per-command pipeline outcomes and no rollback around runtime response errors. This closes no product mutation row; reviewed execution, ambiguity, UI, and remaining Redis matrix gates stay open. |
| Phase 2 — Redis TTL truth | [`139-phase-2-redis-ttl-truth.md`](139-phase-2-redis-ttl-truth.md) | Reviewed 2026-07-17. Redis 7.4.9/8.8.0 under RESP2/RESP3 distinguish missing, persistent, and finite-millisecond key TTL states through a client-independent Rust fact. Redis browsing/editing rows remain open until type views, reviewed TTL mutation, races, service/UI, and the remaining matrix pass. |
| Phase 2 — Redis collection scans | [`141-phase-2-redis-collection-scans.md`](141-phase-2-redis-collection-scans.md) | Reviewed 2026-07-17. Redis 7.4.9/8.8.0 under RESP2/RESP3 prove bounded binary HSCAN, SSCAN, and ZSCAN pages through the shared driver seam; mutation races are subsequently closed by research 142. Redis browsing remains open until complete type views, service/UI integration, and remaining matrix gates pass. |
| Phase 2 — Redis scan mutation races | [`142-phase-2-redis-scan-mutation-races.md`](142-phase-2-redis-scan-mutation-races.md) | Reviewed 2026-07-17. Redis 7.4.9/8.8.0 under RESP2/RESP3 prove full-iteration stable/absent guarantees for SCAN, HSCAN, SSCAN, and ZSCAN during concurrent mutation. Redis browsing remains open until complete type views, service/UI integration, strict transport memory, and remaining matrix gates pass. |
| Phase 2 — Redis timeout/reconnect | [`143-phase-2-redis-timeout-reconnect.md`](143-phase-2-redis-timeout-reconnect.md) | Reviewed 2026-07-17. Redis 7.4.9/8.8.0 under RESP2/RESP3 prove bounded response timeout, confirmed-drop future-call reconnect, logical DB retention, and disposable blocking client identity. TLS/authentication is subsequently closed by research 144; Pub/Sub same-endpoint server replacement by research 148; future-operation credential revocation by research 151. Redis connectivity remains open until DNS change, remaining restart races, service/UI, and remaining matrix gates pass. |
| Phase 2 — Redis Pub/Sub isolation | [`145-phase-2-redis-pubsub-isolation.md`](145-phase-2-redis-pubsub-isolation.md) | Reviewed 2026-07-17. Redis 7.4.9/8.8.0 under RESP2/RESP3 prove bounded binary channel delivery, ordinary-command isolation, explicit overflow, adapter/service client-stop truth, and cancel/drop unsubscribe ownership. Pattern subscriptions are subsequently closed by research 147; reconnect/resubscription and same-endpoint replacement by research 148; TLS/auth composition by research 149. RESP2 pre-decode allocation bounds, DNS races, and UI/native presentation remain open. |
| Phase 2 — Redis reviewed TTL mutation | [`146-phase-2-redis-reviewed-ttl-mutation.md`](146-phase-2-redis-reviewed-ttl-mutation.md) | Reviewed 2026-07-17. Exact-once authorized key TTL changes pass Redis 7.4.9/8.8.0 under RESP2/RESP3 with applied/not-applied truth and explicit unknown post-dispatch outcomes. Conditional expiry UX, hash-field TTLs, general mutation execution, service/UniFFI ownership, and presentation remain open. |
| Phase 2 — Redis TLS/authentication | [`144-phase-2-redis-tls-authentication.md`](144-phase-2-redis-tls-authentication.md) | Reviewed 2026-07-17. Redis 7.4.9/8.8.0 under RESP2/RESP3 prove custom-root TLS, optional required client identity, ACL authentication, hostname verification, plaintext non-fallback, bounded initial authentication stop, TLS reconnect, blocking cancellation, and future-operation credential revocation. The cross-engine Test Connection row remains open until secret resolution, remaining restart behavior, identity/version/elapsed facts, SSH composition, presentation, and remaining engine gates pass. |
| Phase 2 — Redis pattern subscriptions | [`147-phase-2-redis-pattern-subscriptions.md`](147-phase-2-redis-pattern-subscriptions.md) | Reviewed 2026-07-17. Redis 7.4.9/8.8.0 under RESP2/RESP3 prove bounded binary pattern/channel/payload pages through the object-safe adapter, pre-I/O selector/three-column rejection, pre-queue field truncation with original-length truth, client-stop cancellation, and pattern teardown. Reconnect/resubscription with visible delivery gaps is subsequently closed by research 148; TLS composition by research 149. DNS/failure races, strict pre-decode transport allocation, and presentation remain open. |
| Phase 2 — Redis Pub/Sub reconnect | [`148-phase-2-redis-pubsub-reconnect.md`](148-phase-2-redis-pubsub-reconnect.md) | Reviewed 2026-07-17. Redis 7.4.9/8.8.0 under RESP2/RESP3 prove bounded same-endpoint channel/pattern resubscription, an ordered zero-row delivery-gap page before restored binary messages, per-attempt blackhole timeout, bounded exhaustion, and prompt outage cancellation. TLS Pub/Sub composition is subsequently closed by research 149; future-operation credential revocation by research 151; active channel/pattern revocation by research 152; TLS/mTLS replacement by research 153. DNS changes, strict RESP2 pre-decode allocation, and presentation remain open. |
| Phase 2 — Redis TLS Pub/Sub composition | [`149-phase-2-redis-tls-pubsub.md`](149-phase-2-redis-tls-pubsub.md) | Reviewed 2026-07-17. Redis TLS-only 7.4.9/8.8.0 under RESP2/RESP3 prove custom-root and required-mTLS channel/pattern delivery, configured all-channel ACL permission, exact binary pages, authenticated server-observed teardown, active credential revocation, and same-endpoint server replacement. Restricted-channel denial, DNS changes, strict RESP2 pre-decode allocation, and presentation remain open. |
| Phase 2 — Redis Pub/Sub ACL denial boundary | [`150-phase-2-redis-pubsub-acl-denial.md`](150-phase-2-redis-pubsub-acl-denial.md) | Reviewed 2026-07-17. Redis TLS-only 7.4.9/8.8.0 under RESP2/RESP3 prove the server denies a restricted channel under custom-root TLS and required mTLS. The latest redis-rs Pub/Sub setup API erases the denial reply, so adapter-level channel/pattern denial remains an explicit blocker; no administrative preflight, private fork, hand-written RESP, or idle-stream inference is accepted. |
| Phase 2 — Redis live credential revocation | [`151-phase-2-redis-live-credential-revocation.md`](151-phase-2-redis-live-credential-revocation.md) | Reviewed 2026-07-17. Redis TLS-only 7.4.9/8.8.0 under RESP2/RESP3 prove that password rotation plus confirmed user-connection termination makes the next future operation stop with bounded redacted authentication failure; research 152 closes active channel/pattern revocation. Secret re-resolution/new-session UX, DNS changes, strict RESP2 pre-decode allocation, and presentation remain open. |
| Phase 2 — Redis Pub/Sub credential revocation | [`152-phase-2-redis-pubsub-credential-revocation.md`](152-phase-2-redis-pubsub-credential-revocation.md) | Reviewed 2026-07-17. Redis TLS-only 7.4.9/8.8.0 under RESP2/RESP3 prove server-observed active channel and pattern subscriptions stop with bounded redacted authentication failure after independent password rotations and confirmed user-connection termination. Secret re-resolution/new-session UX, DNS changes, restricted denial, strict RESP2 pre-decode allocation, and presentation remain open. |
| Phase 2 — Redis TLS Pub/Sub reconnect | [`153-phase-2-redis-tls-pubsub-reconnect.md`](153-phase-2-redis-tls-pubsub-reconnect.md) | Reviewed 2026-07-17. Redis TLS-only 7.4.9/8.8.0 under RESP2/RESP3 prove channel/pattern resubscription after same-endpoint server replacement under custom roots and required mTLS, with an ordered delivery-gap page before restored binary messages and prompt cancellation; research 154 closes invalid replacement identity/credentials. DNS changes, restricted denial, strict RESP2 pre-decode allocation, and presentation remain open. |
| Phase 2 — Redis TLS Pub/Sub replacement failure | [`154-phase-2-redis-tls-pubsub-replacement-failure.md`](154-phase-2-redis-tls-pubsub-replacement-failure.md) | Reviewed 2026-07-17. Redis TLS-only 7.4.9/8.8.0 under RESP2/RESP3 prove active channel/pattern replacement rejects an untrusted server as redacted connect failure and rotated ACL credentials as redacted authentication failure without a false recovery-gap page. DNS changes, restricted initial denial, strict RESP2 pre-decode allocation, and presentation remain open. |
| Phase 2 — PostgreSQL cancellation completion race | [`155-phase-2-postgresql-cancellation-completion-race.md`](155-phase-2-postgresql-cancellation-completion-race.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 required-mTLS and PostgreSQL 18.4 plaintext fixtures prove SQLSTATE-confirmed cancellation differs from a successfully delivered late cancel after normal completion. A bounded synchronization barrier consumes pending late cancellation before the session is released, and repeated races preserve follow-up usability. Page-delivery races, cancel-time connection loss, notices, parameters, multiple statements, COPY cancellation, ambiguous writes, broader typed values, and presentation remain open. |
| Phase 2 — PostgreSQL cancel transport loss | [`156-phase-2-postgresql-cancel-transport-loss.md`](156-phase-2-postgresql-cancel-transport-loss.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 force-stop fixtures prove server loss before the separate cancel socket opens yields redacted cancellation-transport failure and terminal session connection loss, never false server-confirmed cancellation; typed scalar parameters are subsequently closed by research 157. Post-delivery outcome loss, page-delivery races, reconnect, ambiguous writes, notices, public parameter plans, multiple statements, COPY, typed breadth, and presentation remain open. |
| Phase 2 — PostgreSQL typed parameters | [`157-phase-2-postgresql-typed-parameters.md`](157-phase-2-postgresql-typed-parameters.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 prove prepared UTF-8 text, int8, binary-with-NUL, and boolean parameters through adapter-private client types into one exact bounded typed page. Public reviewed parameter plans/bounds, NULL and structured parameters, statement lifecycle, notices, multiple statements, COPY, reconnect, ambiguous writes, presentation, and UniFFI remain open. |
| Phase 2 — PostgreSQL NULL/array parameters | [`158-phase-2-postgresql-null-array-parameters.md`](158-phase-2-postgresql-null-array-parameters.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 prove declared text NULL remains null and `int4[]` binds through the official client; research 179 subsequently promotes returned arrays from bounded unknown bytes to dimension-preserving `Structured` values. Public parameter plans/count/aggregate bounds, composite/range breadth, statement lifecycle, multiple statements, COPY, reconnect, ambiguous writes, presentation, and UniFFI remain open. |
| Phase 2 — PostgreSQL bounded notices | [`159-phase-2-postgresql-bounded-notices.md`](159-phase-2-postgresql-bounded-notices.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 prove Rust-owned `poll_message` driving, exact bounded severity/SQLSTATE/message, UTF-8-safe truncation with original length, redacted Debug, ordered 64-entry retention, and explicit six-notice overflow. Detail/hint/position, LISTEN notifications, service/UniFFI/UI projection, persistence policy, multiple statements, COPY, reconnect, and ambiguous writes remain open. |
| Phase 2 — PostgreSQL notice detail/hint | [`160-phase-2-postgresql-notice-detail-hint.md`](160-phase-2-postgresql-notice-detail-hint.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 prove optional notice detail/hint retain exact independent bounded values/truncation states, absence truth, and Debug redaction without changing queue overflow. Schema/table/column/position, LISTEN notifications, service/UniFFI/UI projection, persistence policy, multiple statements, COPY, reconnect, and ambiguous writes remain open. |
| Phase 2 — PostgreSQL multiple-statement outcomes | [`161-phase-2-postgresql-multiple-statement-outcomes.md`](161-phase-2-postgresql-multiple-statement-outcomes.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 prove four ordered CREATE/INSERT/UPDATE/SELECT outcomes with stable ordinals, honest command/query kinds, and exact row counts while retaining extended-query binary pages as the only typed row path. Typed per-statement pages, parser boundaries, transaction/partial-failure semantics, public bounds, cancellation, notice association, service/store/history/UI/UniFFI, and arbitrary execution remain open. |
| Phase 2 — PostgreSQL bounded COPY streaming | [`162-phase-2-postgresql-bounded-copy-streaming.md`](162-phase-2-postgresql-bounded-copy-streaming.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 prove bounded pull-driven COPY OUT chunks with exact order/offset/bytes and bounded backpressured COPY IN with server-confirmed rows, explicit pre-I/O/input and streaming/output limit failures, payload-redacted Debug, malformed-input failure, and session recovery. Arbitrary reviewed plans, file effects, cancellation/progress, partial-file policy, format/encoding breadth, TLS/loss matrices, service/history/UI/UniFFI, and clean-machine transfer evidence remain open. |
| Phase 2 — PostgreSQL ambiguous write | [`163-phase-2-postgresql-ambiguous-write.md`](163-phase-2-postgresql-ambiguous-write.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 prove a dispatched write whose response observation times out remains explicitly unknown even when an independent session later sees one durable row; the original session drains, remains usable, and never replays the write. Connection-loss timing, TLS loss, transaction commit ambiguity, reconnect ownership, reviewed plans, service/history/UI/UniFFI remain open. |
| Phase 2 — PostgreSQL ambiguous commit | [`164-phase-2-postgresql-ambiguous-commit.md`](164-phase-2-postgresql-ambiguous-commit.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 prove an explicit transaction whose deferred commit trigger outlives client response observation remains unknown, later commits exactly one durable row, drains on the original session, and never replays. Connection-loss timing, TLS loss, rollback observation, reconnect ownership, reviewed plans, conflicts, service/history/UI/UniFFI remain open. |
| Phase 2 — PostgreSQL commit transport loss | [`165-phase-2-postgresql-commit-transport-loss.md`](165-phase-2-postgresql-commit-transport-loss.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 prove activity-gated server loss during deferred COMMIT yields unknown, terminal old sessions, explicit same-data-directory restart through a freshly resolved host mapping, observed rollback, and no replay. Pre/mid-dispatch loss, post-commit/pre-response loss, TLS loss, shared-service reconnect ownership, reviewed plans, conflicts, history/UI/UniFFI remain open. |
| Phase 2 — PostgreSQL mTLS commit loss | [`166-phase-2-postgresql-mtls-commit-loss.md`](166-phase-2-postgresql-mtls-commit-loss.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 prove active-COMMIT transport loss under verified custom roots and required client identity remains unknown, terminates old TLS sessions, rejects plaintext recovery, refreshes endpoint facts, revalidates mTLS, observes rollback, and never replays. Pre/mid-dispatch and post-commit/pre-response loss, identity rotation, shared-service reconnect ownership, reviewed plans, conflicts, history/UI/UniFFI remain open. |
| Phase 2 — PostgreSQL complex raw values | [`167-phase-2-postgresql-complex-raw-values.md`](167-phase-2-postgresql-complex-raw-values.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 preserve bounded JSON, JSONB, `int4range`, and anonymous-record binary payloads with exact type and truncation truth; large `bytea` remains binary. Research 168 promotes JSON/JSONB, research 180 ranges, and research 182 composites to `Structured`; strict pre-driver field allocation, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL JSON projection | [`168-phase-2-postgresql-json-projection.md`](168-phase-2-postgresql-json-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 project JSON/JSONB into deterministic compact bounded `Structured` values with arbitrary-precision number, malformed/version-invalid, and 8 MiB pre-DOM allocation truth. Raw inspector access, strict pre-driver field allocation, other structured decoders, service/UI, and UniFFI remain open. |
| Phase 2 — Redis atomic revocation fixture | [`170-phase-2-redis-atomic-revocation-fixture.md`](170-phase-2-redis-atomic-revocation-fixture.md) | Reviewed 2026-07-17. Multi-user live revocation dispatches both administrative connection kills in one pipeline, removing stale reconnect interference while retaining independent server counts. This hardens evidence but closes no additional product capability. |
| Phase 2 — Redis subscription connect policy | [`171-phase-2-redis-subscription-connect-policy.md`](171-phase-2-redis-subscription-connect-policy.md) | Reviewed 2026-07-17. Initial and replacement RESP2/RESP3 Pub/Sub generations share bounded cancellable connection attempts; required-TLS deadline exhaustion deterministically maps to `Connect`, plaintext blackholes remain `Timeout`, and initial setup emits no recovery gap. DNS change, strict pre-decode allocation, restricted denial, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL numeric decoder | [`172-phase-2-postgresql-numeric-decoder.md`](172-phase-2-postgresql-numeric-decoder.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 decode finite arbitrary-precision numeric values, declared scale/trailing zeros, scaled zero, NaN, and infinities into exact bounded `Decimal`; malformed and over-cell-limit values retain invalid/unknown truth. Other typed breadth, strict pre-driver allocation, service/UI, and UniFFI remain open. |
| Phase 2 — Redis administrative readiness budget | [`173-phase-2-redis-admin-readiness-budget.md`](173-phase-2-redis-admin-readiness-budget.md) | Reviewed 2026-07-17. Raw TLS fixture administration overrides redis-rs's 500 ms response default with explicit bounded connection/response budgets and requires PING/PONG command readiness. This hardens the full real-server evidence but closes no additional product capability. |
| Phase 2 — PostgreSQL UUID decoder | [`174-phase-2-postgresql-uuid-decoder.md`](174-phase-2-postgresql-uuid-decoder.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 project representative, nil, and maximum UUID binary values into canonical lowercase bounded `Text`, with exact 36-byte truncation and malformed-length truth. Other typed breadth, service/UI, and UniFFI remain open. |
| Phase 2 — temporal value contract | [`175-phase-2-temporal-value-contract.md`](175-phase-2-temporal-value-contract.md) | Reviewed 2026-07-17. The shared owned-value and immutable-page contracts now carry a first-class bounded UTF-8 `Temporal` kind, including truncation validation and structured-projection support. Database decoders, canonical temporal forms, editors, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL temporal decoder | [`176-phase-2-postgresql-temporal-decoder.md`](176-phase-2-postgresql-temporal-decoder.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 binary date, time, timestamp, and timestamptz values now project into bounded canonical `Temporal`, preserving microseconds, 24:00, UTC instant truth, and infinities. Research 177 subsequently closes `timetz`, interval, and BC/expanded-year result decoding; editors, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL temporal completion | [`177-phase-2-postgresql-temporal-completion.md`](177-phase-2-postgresql-temporal-completion.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now project `timetz`, exact independent interval components and sentinels, astronomical BC dates, and signed expanded years into bounded canonical `Temporal`. Research 179/180 subsequently compose temporal values through arrays/ranges; editors, parameters/mutations, service/UI, and UniFFI remain open. |
| Phase 2 — ClickHouse temporal projection | [`178-phase-2-clickhouse-temporal-projection.md`](178-phase-2-clickhouse-temporal-projection.md) | Reviewed 2026-07-17. ClickHouse 25.8/26.3 LTS now project Date/Date32/DateTime/DateTime64 into bounded canonical `Temporal`, preserve declared fractional scale, normalize epoch instants to UTC, retain timezone metadata, and quote recursive temporal containers. Editors, mutation/input paths, UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL array projection | [`179-phase-2-postgresql-array-projection.md`](179-phase-2-postgresql-array-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now project generic binary arrays into bounded canonical `Structured` values while preserving dimensions, lower bounds, row-major nesting, NULL elements, and supported scalar kinds. Malformed structure becomes `Invalid`; valid unsupported or over-budget arrays remain whole-value `Unknown`. Research 182/183 subsequently close composite/domain elements; editors, public parameters, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL range projection | [`180-phase-2-postgresql-range-projection.md`](180-phase-2-postgresql-range-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now project generic binary ranges into bounded canonical `Structured` values with explicit empty, unbounded, inclusive, and exclusive truth across integer, numeric, date, and timestamp subtypes. Invalid flags/lengths become `Invalid`; unsupported subtype projection remains whole-value `Unknown`. Research 181 composes ranges through multiranges and research 182/183 close composite/domain decoding; editors, public parameters, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL multirange projection | [`181-phase-2-postgresql-multirange-projection.md`](181-phase-2-postgresql-multirange-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now project generic binary multiranges into ordered bounded canonical `Structured` members using the range contract, including empty, integer, unbounded, numeric, and date values. Invalid counts/lengths/member payloads become `Invalid`; over-budget or unsupported members remain whole-value `Unknown`. Research 182/183 subsequently close composite/domain decoding; editors, public parameters, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL composite projection | [`182-phase-2-postgresql-composite-projection.md`](182-phase-2-postgresql-composite-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now project named composites and anonymous records into bounded canonical `Structured` fields with names/null names, exact OID/type identity, NULL truth, nested arrays/ranges, strict framing, a 1,664-field ceiling, and a shared 64-level structured nesting cap. Unknown anonymous OIDs and over-budget structures remain whole-value `Unknown`; malformed values become `Invalid`. Research 183 subsequently closes domain decoding; editors, public parameters, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL domain projection | [`183-phase-2-postgresql-domain-projection.md`](183-phase-2-postgresql-domain-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now decode scalar, nested-domain, array, range, and composite domains through underlying semantic contracts while composite fields retain exact domain name/OID identity. Invalid/unknown fallback retains the outer domain identity and domains consume the shared recursion budget. PostgreSQL RowDescription flattens top-level domain expressions/columns to base types; this protocol truth is explicit. Editors, public parameters, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL enum projection | [`184-phase-2-postgresql-enum-projection.md`](184-phase-2-postgresql-enum-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now project ASCII and Unicode enum labels into bounded UTF-8-safe `Text` with exact user-defined column type identity. Invalid UTF-8 or labels absent from pinned catalog metadata become `Invalid` with enum identity. Editors/schema metadata, public parameters, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL network projection | [`185-phase-2-postgresql-network-projection.md`](185-phase-2-postgresql-network-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now project `inet`, `cidr`, `macaddr`, and `macaddr8` binary values into bounded canonical `Text` with exact type identity. Family, prefix, CIDR flag/network truth, address length, framing, and MAC width are strict; malformed values become `Invalid`. Editors/schema metadata, public parameters, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL bit-string projection | [`186-phase-2-postgresql-bit-string-projection.md`](186-phase-2-postgresql-bit-string-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now project fixed `bit` and variable `varbit` binary values into bounded canonical `Text` while retaining exact type identity. Signed count, exact payload size, trailing bytes, and unused padding bits are strict; malformed values become `Invalid`. Editors/type modifiers, public parameters, service/UI, and UniFFI remain open. |
