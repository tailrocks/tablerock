# Functional Parity Ledger

## Decision

TableRock targets a complete PostgreSQL, ClickHouse, and Redis workbench with
terminal and native macOS clients. TablePro is evidence that broad workflows
exist, not a visual or implementation specification.

“Parity” means an operator can accomplish the same relevant database task with
equivalent safety, state visibility, and failure handling. It does **not** mean
the same layout, words, geometry, colors, icons, shortcuts, assets, or internal
architecture. TableRock requirements come from this repository, official
database/client/platform documentation, and direct tests. Screen-level behavior
is specified in [`docs/product/`](../product/README.md); this ledger is the
capability checklist those screens must satisfy.

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
| URL import | Parity | Parse supported database URLs into a reviewable draft; credentials remain transient unless the operator selects a secret destination | Parser fixtures + ImportUrl dialog (evidence 289); percent encoding, TLS, hostile reject |
| External URL open | Later | Confirm target and safety before opening a temporary or matching saved session | Deep-link hostile-input tests + OPEN confirm temporary connect (evidence 296) |
| Test connection | Core | Show server identity/version, TLS outcome, elapsed time, and redacted diagnostics without saving | Real TLS/auth fixtures for all engines |
| Temporary connection | Core | Connect without persisting profile or secret | Relaunch proves no durable profile/secret remains |
| Profile organization | Parity | Groups, tags, favorites, ordering, filters, and environment/safety markers without copied reference presentation | Group UI and environment-tag projection in [connections.md](../product/connections.md); Phase 2 profile/group/tag/filter/search evidence in the [profiles group](../evidence/README.md#phase-2--profiles); live health projection, resolution, UI, and unrelated-entity retention remain required |
| Environment tag | Core | Per-profile environment label (production/staging/development/custom) shown in list, editor, context bar, and tabs; production renders a persistent warning treatment, never color alone | Snapshot fixtures across all four surfaces |
| Context switcher | Core | Switch connection/database/schema/logical database using engine-correct semantics | Driver contract tests prove no fake cross-engine abstraction |
| Health and reconnect | Core | Visible state, bounded backoff, authentication stop condition, context restoration, and no automatic ambiguous-write retry | Disconnect/reconnect race harness |
| Startup actions | Later | Reviewed, bounded startup SQL/commands with explicit reconnect behavior | Core + PG/CH/Redis executors + persist/TUI + Write/Dangerous review (evidence 270–274, 282) |
| SSH transport | Core | Rust `russh` adapter below clients with host-key verification, known-hosts, key/agent/password auth, local-forward, cancel cleanup | Real bastion matrix (evidence 260–269); keepalive/reconnect polish and profile-persisted agent preference remain |
| Cloud-provider proxy/identity | Excluded | No vendor proxy/identity workflow in this three-engine program | Product-boundary test and documentation |

## Workbench and navigation

| Capability | Status | TableRock requirement | Acceptance evidence |
|---|---|---|---|
| Lazy catalog | Core | Engine-native hierarchy including tables, views, and PostgreSQL functions/routines with name and signature; loading/stale/error nodes, subtree refresh, filtering with ancestors preserved | Snapshot contract + driver subtree listing proven ([198](../evidence/phase-2/core/198-phase-2-catalog-listing.md): PG tables/views/functions+signatures, CH objects, Redis logical DBs); ancestor-preserving filter and UI projection remain |
| Object tabs | Core | Preview/pinned/durable object tabs with independent context and state; the same object may open in several tabs with independent sort/filter/columns/staged changes | Restore, close-policy, and multi-instance independence tests; CloseOthers (401); CloseLeft/CloseRight (evidence 428); CloseAll (evidence 434) |
| Query/command tabs | Core | Independent text, cursor, context, results, errors, history, and running operation | Phase 2 command-scope/operation evidence in the [core group](../evidence/README.md#phase-2--core-contracts-and-services); execution text, multi-tab behavior, release-profile budgets, and remaining server races remain required |
| Result tabs | Parity | One result per statement/operation, pinning, completion summaries, failure and partial-result states | Phase 2 safe-diagnostic taxonomy in the [core group](../evidence/README.md#phase-2--core-contracts-and-services); multi-statement and real-driver mapping fixtures remain required |
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
| Virtualized grid | Core | Render only resident rows/columns; stable two-axis navigation and placeholders; no I/O from render | VirtualGrid + DataGridModel (211–214); resident-scroll no-I/O; million-row totals + resident window unit proof (evidence 355); CI frame-time microbench optional |
| Typed display | Core | Distinguish NULL, empty, whitespace, zero, false, bytes, structured containers, unknown, invalid, and truncated values | Phase 2 owned-value contract plus ClickHouse 25.8/26.3 fixtures (see [core](../evidence/README.md#phase-2--core-contracts-and-services) and [ClickHouse](../evidence/README.md#phase-2--clickhouse-driver) groups); Phase 4 TUI distinction classes + glyph/text treatments (evidence 211) |
| Column controls | Parity | Show/hide, reorder, width, fit, format, one-action reset, and stable per-table preferences | Toggle/reset + persist (225); ColL/ColR + Col± (347); ColFit/ColFitA (352); ColSolo (390); ColAll (392); ColInv (evidence 395); ColRstW widths-only (evidence 425); ColHome/ColEnd (evidence 426); ColEqW (evidence 429); ColPk identity solo (evidence 441); GoPk/GoPkLast (evidence 442, 446); ColJson (evidence 445); ColHideE empty hide (evidence 447); Left/Right skip hidden (evidence 452); ColSnap (evidence 453) |
| Sorting | Core | Server sort for table browsing, explicit provenance, multi-column order, no unsafe SQL concatenation | Phase 5 `BrowsePlan` + CycleSort (223–225); PushSort/PopSort multi-key + sort chip bar (evidence 370); SortInv/SortRot/Sort1/SortPri/SortInvA (418–422); CopySort (evidence 430); SortSwap (evidence 435); SortRotR (evidence 436); SortRev (evidence 443); hostile identifier suite in browse_plan tests |
| Filtering | Core | Filter bar with typed per-column conditions plus a raw-WHERE mode; clearly labeled resident-page quick filter; saved presets later | Parameterized filters + page-local (223–225); presets (306–310); chip bar (349); null/pop (358); LIKE/NE/compare (359–360); EditRawWhere (evidence 361); Starts/Ends LIKE affix (evidence 423); ShiftFilt oldest chip (evidence 424); IStarts/IEnds ILIKE (evidence 431); ClrFiltS keep-sort (evidence 432); CopyFilt filters-only (evidence 437); RevFilt (evidence 440); PromoFilt/DemoFilt (evidence 448–449) |
| Selection/copy | Parity | Cell/range/row selection and safe TSV/CSV/JSON/Markdown/SQL-INSERT/SQL-UPDATE projections; INSERT/UPDATE require base-table identity facts | Phase 5 six-format pure formatters + OSC 52 (224); CopyCell/hex/row formats (354–355, 375, 380); CopyPick scope+format dialog (evidence 393); identity-gated SQL tests; CopySch/CopyTblN parts (evidence 427); CopyColN (evidence 433); CopySql cell literal (evidence 438); CopyQid (evidence 439); CopyProg (evidence 444); CopyHid (evidence 450); CopyTok (evidence 451) |
| Paging | Core | Bounded server pages for objects and bounded streams for arbitrary queries; totals may be estimated/unknown | Phase 2 page/result-store contracts; Phase 4 CLI pump-and-store + FetchPage pin (evidence 214); Docker 2,500-row / 500-page browse; 10k query cap; driver/store race integration for full operation matrix remains open |
| Row/value inspector | Core | Full typed values, raw bytes/hex, JSON text/tree, metadata, and stale state | Panel text/hex/metadata (213); JSON tree (339); multi-line binary hex dump (356); staged/original (374); Tree± expand (evidence 379) |
| Type-specific editors | Parity | Bool, number, temporal, enum, JSON, bytes, array/map/tuple, and explicit unknown fallback | Paste + heuristics (230); TogBool/SetNull; Today/Now (346); Day± (365); Mon±/PickDate (369); Num± (351); FmtJson/CmpJson (353); enum picker optional |
| Stable editability | Core | Only results with proven base object and stable identity are editable | `EditabilityFacts` + browse PK proof; ReadOnly profile blocks staging (evidence 228–229) |
| Staged changes | Core | Inserts/updates/deletes remain local, visible (per-row/per-cell markers), undoable, reviewable, and discardable until apply | `MutationDraftModel` markers/undo/discard + dirty tab; apply clears on success (228, 230); InsRow/DupRow (371); draft paint (373); UnstageCell/UnstageRow (evidence 391) |
| Operation preview | Core | Preview lists the exact parameterized operations; execution uses the typed plan, never reparsed display text | Review lines from typed plan (`mutation_plan_build`); apply rebuilds typed plan from drafts (evidence 229–230) |
| Conflict handling | Core | PostgreSQL conflicts roll back; ClickHouse and Redis expose their true non-transactional outcomes | PG ≠1-row conflict → ROLLBACK + keep staged; Docker suite (evidence 227, 230) |
| Foreign-key navigation | Parity | PostgreSQL relationship lookup and navigation with explicit unavailable state elsewhere | FollowForeignKey → filtered browse (231); multi-column FK follow (evidence 326) |

## Schema, administration, and data movement

| Capability | Status | TableRock requirement | Acceptance evidence |
|---|---|---|---|
| Structure inspection | Core | Columns, keys/indexes, constraints, engine facts, and DDL/raw metadata | Columns (231); indexes/constraints (324); reconstructed CREATE TABLE dump in structure panel (evidence 364) |
| Structure editing | Later | Capability-gated reviewed DDL; PostgreSQL first, ClickHouse-specific forms, no Redis fiction | Destructive-operation and rollback/outcome tests |
| Table operations | Parity | Refresh, rename where valid, truncate/drop, maintenance/optimize, and copied DDL behind typed safety gates | Truncate/drop (232); rename (340); VACUUM/ANALYZE (341); CopyStructureDdl CREATE TABLE (evidence 342) |
| Import | Parity | Streaming CSV/JSON and reviewed SQL where meaningful; mapping, transaction/outcome policy, progress, cancel | Malformed input, formula, encoding, and partial-failure fixtures |
| Export | Parity | Streaming CSV/JSON plus engine-appropriate SQL; atomic destination and cancellation cleanup | Constant-memory and partial-file tests |
| Backup/restore | Later | PostgreSQL tool integration with version checks, progress, cancel, and secret-safe process invocation | Real `pg_dump`/`pg_restore` matrix |
| ER relationships | Later | PostgreSQL relationship graph; terminal tree/list first, native diagram later | Cycles, large graph, and missing-FK tests |
| Server dashboard | Parity | Current bounded health/activity snapshots, cancel/kill only through explicit capability and confirmation | Activity snapshot (232); cancel/terminate + permission-denied (evidence 327) |
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
[03-database-capabilities.md](database-capabilities.md).

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
| Appearance | Liquid Glass design language (macOS 26+): glass toolbar/sidebar/transient layer, opaque content surfaces, native light/dark and accessibility degradation, not copied reference colors |
| Performance | Coarse immutable pages and events; never one Rust/IPC call per cell |

The ownership and transport decision is in
[12-native-macos-path.md](native-macos-path.md) and
[14-shared-client-contract.md](shared-client-contract.md).

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
| Phase 1 — TermRock and TUI foundation | [`45-phase-1-exit-report.md`](../evidence/phase-1/45-phase-1-exit-report.md) | Reviewed 2026-07-16. The empty shell closes no Core, Parity, or Later product capability. All unimplemented rows remain visible blockers for their owning parity claims; fixed exclusions are unchanged. |
| Phase 2 — core identity/revision tracer | [`46-phase-2-core-identity.md`](../evidence/phase-2/core/46-phase-2-core-identity.md) | Reviewed 2026-07-16. Canonical identity and stale/gap classification infrastructure closes no user capability. Identity namespace provisioning and every remaining Phase 2 contract/storage/driver row stay visible blockers. |
| Phase 2 — persistence backup/restore | [`135-phase-2-persistence-backup-restore.md`](../evidence/phase-2/persistence/135-phase-2-persistence-backup-restore.md) | Reviewed 2026-07-17. The bounded offline state-store backup and restore primitive closes no PostgreSQL tool-level backup/restore capability. Phase 10 `pg_dump`/`pg_restore`, destructive operator replacement UX, and remaining Phase 2 storage fault/deployment gates stay visible blockers. |
| Phase 2 — PostgreSQL TLS/client identity | [`136-phase-2-postgresql-tls-identity.md`](../evidence/phase-2/postgres/136-phase-2-postgresql-tls-identity.md) | Reviewed 2026-07-17. Verified custom-root TLS and mTLS pass on PostgreSQL 17.10/18.4, but the cross-engine Test Connection capability remains open until all engines, resolved secret sourcing, elapsed/server identity facts, and presentation flows pass. |
| Phase 2 — Redis pipeline partial failures | [`138-phase-2-redis-pipeline-partial-failure.md`](../evidence/phase-2/redis/138-phase-2-redis-pipeline-partial-failure.md) | Reviewed 2026-07-17. Redis 7.4.9/8.8.0 under RESP2/RESP3 prove per-command pipeline outcomes and no rollback around runtime response errors. This closes no product mutation row; reviewed execution, ambiguity, UI, and remaining Redis matrix gates stay open. |
| Phase 2 — Redis TTL truth | [`139-phase-2-redis-ttl-truth.md`](../evidence/phase-2/redis/139-phase-2-redis-ttl-truth.md) | Reviewed 2026-07-17. Redis 7.4.9/8.8.0 under RESP2/RESP3 distinguish missing, persistent, and finite-millisecond key TTL states through a client-independent Rust fact. Redis browsing/editing rows remain open until type views, reviewed TTL mutation, races, service/UI, and the remaining matrix pass. |
| Phase 2 — Redis collection scans | [`141-phase-2-redis-collection-scans.md`](../evidence/phase-2/redis/141-phase-2-redis-collection-scans.md) | Reviewed 2026-07-17. Redis 7.4.9/8.8.0 under RESP2/RESP3 prove bounded binary HSCAN, SSCAN, and ZSCAN pages through the shared driver seam; mutation races are subsequently closed by research 142. Redis browsing remains open until complete type views, service/UI integration, and remaining matrix gates pass. |
| Phase 2 — Redis scan mutation races | [`142-phase-2-redis-scan-mutation-races.md`](../evidence/phase-2/redis/142-phase-2-redis-scan-mutation-races.md) | Reviewed 2026-07-17. Redis 7.4.9/8.8.0 under RESP2/RESP3 prove full-iteration stable/absent guarantees for SCAN, HSCAN, SSCAN, and ZSCAN during concurrent mutation. Redis browsing remains open until complete type views, service/UI integration, strict transport memory, and remaining matrix gates pass. |
| Phase 2 — Redis timeout/reconnect | [`143-phase-2-redis-timeout-reconnect.md`](../evidence/phase-2/redis/143-phase-2-redis-timeout-reconnect.md) | Reviewed 2026-07-17. Redis 7.4.9/8.8.0 under RESP2/RESP3 prove bounded response timeout, confirmed-drop future-call reconnect, logical DB retention, and disposable blocking client identity. TLS/authentication is subsequently closed by research 144; Pub/Sub same-endpoint server replacement by research 148; future-operation credential revocation by research 151. Redis connectivity remains open until DNS change, remaining restart races, service/UI, and remaining matrix gates pass. |
| Phase 2 — Redis Pub/Sub isolation | [`145-phase-2-redis-pubsub-isolation.md`](../evidence/phase-2/redis/145-phase-2-redis-pubsub-isolation.md) | Reviewed 2026-07-17. Redis 7.4.9/8.8.0 under RESP2/RESP3 prove bounded binary channel delivery, ordinary-command isolation, explicit overflow, adapter/service client-stop truth, and cancel/drop unsubscribe ownership. Pattern subscriptions are subsequently closed by research 147; reconnect/resubscription and same-endpoint replacement by research 148; TLS/auth composition by research 149. RESP2 pre-decode allocation bounds, DNS races, and UI/native presentation remain open. |
| Phase 2 — Redis reviewed TTL mutation | [`146-phase-2-redis-reviewed-ttl-mutation.md`](../evidence/phase-2/redis/146-phase-2-redis-reviewed-ttl-mutation.md) | Reviewed 2026-07-17. Exact-once authorized key TTL changes pass Redis 7.4.9/8.8.0 under RESP2/RESP3 with applied/not-applied truth and explicit unknown post-dispatch outcomes. Conditional expiry UX, hash-field TTLs, general mutation execution, service/UniFFI ownership, and presentation remain open. |
| Phase 2 — Redis TLS/authentication | [`144-phase-2-redis-tls-authentication.md`](../evidence/phase-2/redis/144-phase-2-redis-tls-authentication.md) | Reviewed 2026-07-17. Redis 7.4.9/8.8.0 under RESP2/RESP3 prove custom-root TLS, optional required client identity, ACL authentication, hostname verification, plaintext non-fallback, bounded initial authentication stop, TLS reconnect, blocking cancellation, and future-operation credential revocation. The cross-engine Test Connection row remains open until secret resolution, remaining restart behavior, identity/version/elapsed facts, SSH composition, presentation, and remaining engine gates pass. |
| Phase 2 — Redis pattern subscriptions | [`147-phase-2-redis-pattern-subscriptions.md`](../evidence/phase-2/redis/147-phase-2-redis-pattern-subscriptions.md) | Reviewed 2026-07-17. Redis 7.4.9/8.8.0 under RESP2/RESP3 prove bounded binary pattern/channel/payload pages through the object-safe adapter, pre-I/O selector/three-column rejection, pre-queue field truncation with original-length truth, client-stop cancellation, and pattern teardown. Reconnect/resubscription with visible delivery gaps is subsequently closed by research 148; TLS composition by research 149. DNS/failure races, strict pre-decode transport allocation, and presentation remain open. |
| Phase 2 — Redis Pub/Sub reconnect | [`148-phase-2-redis-pubsub-reconnect.md`](../evidence/phase-2/redis/148-phase-2-redis-pubsub-reconnect.md) | Reviewed 2026-07-17. Redis 7.4.9/8.8.0 under RESP2/RESP3 prove bounded same-endpoint channel/pattern resubscription, an ordered zero-row delivery-gap page before restored binary messages, per-attempt blackhole timeout, bounded exhaustion, and prompt outage cancellation. TLS Pub/Sub composition is subsequently closed by research 149; future-operation credential revocation by research 151; active channel/pattern revocation by research 152; TLS/mTLS replacement by research 153. DNS changes, strict RESP2 pre-decode allocation, and presentation remain open. |
| Phase 2 — Redis TLS Pub/Sub composition | [`149-phase-2-redis-tls-pubsub.md`](../evidence/phase-2/redis/149-phase-2-redis-tls-pubsub.md) | Reviewed 2026-07-17. Redis TLS-only 7.4.9/8.8.0 under RESP2/RESP3 prove custom-root and required-mTLS channel/pattern delivery, configured all-channel ACL permission, exact binary pages, authenticated server-observed teardown, active credential revocation, and same-endpoint server replacement. Restricted-channel denial, DNS changes, strict RESP2 pre-decode allocation, and presentation remain open. |
| Phase 2 — Redis Pub/Sub ACL denial boundary | [`150-phase-2-redis-pubsub-acl-denial.md`](../evidence/phase-2/redis/150-phase-2-redis-pubsub-acl-denial.md) | Reviewed 2026-07-17. Redis TLS-only 7.4.9/8.8.0 under RESP2/RESP3 prove the server denies a restricted channel under custom-root TLS and required mTLS. The latest redis-rs Pub/Sub setup API erases the denial reply, so adapter-level channel/pattern denial remains an explicit blocker; no administrative preflight, private fork, hand-written RESP, or idle-stream inference is accepted. |
| Phase 2 — Redis live credential revocation | [`151-phase-2-redis-live-credential-revocation.md`](../evidence/phase-2/redis/151-phase-2-redis-live-credential-revocation.md) | Reviewed 2026-07-17. Redis TLS-only 7.4.9/8.8.0 under RESP2/RESP3 prove that password rotation plus confirmed user-connection termination makes the next future operation stop with bounded redacted authentication failure; research 152 closes active channel/pattern revocation. Secret re-resolution/new-session UX, DNS changes, strict RESP2 pre-decode allocation, and presentation remain open. |
| Phase 2 — Redis Pub/Sub credential revocation | [`152-phase-2-redis-pubsub-credential-revocation.md`](../evidence/phase-2/redis/152-phase-2-redis-pubsub-credential-revocation.md) | Reviewed 2026-07-17. Redis TLS-only 7.4.9/8.8.0 under RESP2/RESP3 prove server-observed active channel and pattern subscriptions stop with bounded redacted authentication failure after independent password rotations and confirmed user-connection termination. Secret re-resolution/new-session UX, DNS changes, restricted denial, strict RESP2 pre-decode allocation, and presentation remain open. |
| Phase 2 — Redis TLS Pub/Sub reconnect | [`153-phase-2-redis-tls-pubsub-reconnect.md`](../evidence/phase-2/redis/153-phase-2-redis-tls-pubsub-reconnect.md) | Reviewed 2026-07-17. Redis TLS-only 7.4.9/8.8.0 under RESP2/RESP3 prove channel/pattern resubscription after same-endpoint server replacement under custom roots and required mTLS, with an ordered delivery-gap page before restored binary messages and prompt cancellation; research 154 closes invalid replacement identity/credentials. DNS changes, restricted denial, strict RESP2 pre-decode allocation, and presentation remain open. |
| Phase 2 — Redis TLS Pub/Sub replacement failure | [`154-phase-2-redis-tls-pubsub-replacement-failure.md`](../evidence/phase-2/redis/154-phase-2-redis-tls-pubsub-replacement-failure.md) | Reviewed 2026-07-17. Redis TLS-only 7.4.9/8.8.0 under RESP2/RESP3 prove active channel/pattern replacement rejects an untrusted server as redacted connect failure and rotated ACL credentials as redacted authentication failure without a false recovery-gap page. DNS changes, restricted initial denial, strict RESP2 pre-decode allocation, and presentation remain open. |
| Phase 2 — PostgreSQL cancellation completion race | [`155-phase-2-postgresql-cancellation-completion-race.md`](../evidence/phase-2/postgres/155-phase-2-postgresql-cancellation-completion-race.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 required-mTLS and PostgreSQL 18.4 plaintext fixtures prove SQLSTATE-confirmed cancellation differs from a successfully delivered late cancel after normal completion. A bounded synchronization barrier consumes pending late cancellation before the session is released, and repeated races preserve follow-up usability. Page-delivery races, cancel-time connection loss, notices, parameters, multiple statements, COPY cancellation, ambiguous writes, broader typed values, and presentation remain open. |
| Phase 2 — PostgreSQL cancel transport loss | [`156-phase-2-postgresql-cancel-transport-loss.md`](../evidence/phase-2/postgres/156-phase-2-postgresql-cancel-transport-loss.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 force-stop fixtures prove server loss before the separate cancel socket opens yields redacted cancellation-transport failure and terminal session connection loss, never false server-confirmed cancellation; typed scalar parameters are subsequently closed by research 157. Post-delivery outcome loss, page-delivery races, reconnect, ambiguous writes, notices, public parameter plans, multiple statements, COPY, typed breadth, and presentation remain open. |
| Phase 2 — PostgreSQL typed parameters | [`157-phase-2-postgresql-typed-parameters.md`](../evidence/phase-2/postgres/157-phase-2-postgresql-typed-parameters.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 prove prepared UTF-8 text, int8, binary-with-NUL, and boolean parameters through adapter-private client types into one exact bounded typed page. Public reviewed parameter plans/bounds, NULL and structured parameters, statement lifecycle, notices, multiple statements, COPY, reconnect, ambiguous writes, presentation, and UniFFI remain open. |
| Phase 2 — PostgreSQL NULL/array parameters | [`158-phase-2-postgresql-null-array-parameters.md`](../evidence/phase-2/postgres/158-phase-2-postgresql-null-array-parameters.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 prove declared text NULL remains null and `int4[]` binds through the official client; research 179 subsequently promotes returned arrays from bounded unknown bytes to dimension-preserving `Structured` values. Public parameter plans/count/aggregate bounds, composite/range breadth, statement lifecycle, multiple statements, COPY, reconnect, ambiguous writes, presentation, and UniFFI remain open. |
| Phase 2 — PostgreSQL bounded notices | [`159-phase-2-postgresql-bounded-notices.md`](../evidence/phase-2/postgres/159-phase-2-postgresql-bounded-notices.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 prove Rust-owned `poll_message` driving, exact bounded severity/SQLSTATE/message, UTF-8-safe truncation with original length, redacted Debug, ordered 64-entry retention, and explicit six-notice overflow. Detail/hint/position, LISTEN notifications, service/UniFFI/UI projection, persistence policy, multiple statements, COPY, reconnect, and ambiguous writes remain open. |
| Phase 2 — PostgreSQL notice detail/hint | [`160-phase-2-postgresql-notice-detail-hint.md`](../evidence/phase-2/postgres/160-phase-2-postgresql-notice-detail-hint.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 prove optional notice detail/hint retain exact independent bounded values/truncation states, absence truth, and Debug redaction without changing queue overflow. Schema/table/column/position, LISTEN notifications, service/UniFFI/UI projection, persistence policy, multiple statements, COPY, reconnect, and ambiguous writes remain open. |
| Phase 2 — PostgreSQL multiple-statement outcomes | [`161-phase-2-postgresql-multiple-statement-outcomes.md`](../evidence/phase-2/postgres/161-phase-2-postgresql-multiple-statement-outcomes.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 prove four ordered CREATE/INSERT/UPDATE/SELECT outcomes with stable ordinals, honest command/query kinds, and exact row counts while retaining extended-query binary pages as the only typed row path. Typed per-statement pages, parser boundaries, transaction/partial-failure semantics, public bounds, cancellation, notice association, service/store/history/UI/UniFFI, and arbitrary execution remain open. |
| Phase 2 — PostgreSQL bounded COPY streaming | [`162-phase-2-postgresql-bounded-copy-streaming.md`](../evidence/phase-2/postgres/162-phase-2-postgresql-bounded-copy-streaming.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 prove bounded pull-driven COPY OUT chunks with exact order/offset/bytes and bounded backpressured COPY IN with server-confirmed rows, explicit pre-I/O/input and streaming/output limit failures, payload-redacted Debug, malformed-input failure, and session recovery. Arbitrary reviewed plans, file effects, cancellation/progress, partial-file policy, format/encoding breadth, TLS/loss matrices, service/history/UI/UniFFI, and clean-machine transfer evidence remain open. |
| Phase 2 — PostgreSQL ambiguous write | [`163-phase-2-postgresql-ambiguous-write.md`](../evidence/phase-2/postgres/163-phase-2-postgresql-ambiguous-write.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 prove a dispatched write whose response observation times out remains explicitly unknown even when an independent session later sees one durable row; the original session drains, remains usable, and never replays the write. Connection-loss timing, TLS loss, transaction commit ambiguity, reconnect ownership, reviewed plans, service/history/UI/UniFFI remain open. |
| Phase 2 — PostgreSQL ambiguous commit | [`164-phase-2-postgresql-ambiguous-commit.md`](../evidence/phase-2/postgres/164-phase-2-postgresql-ambiguous-commit.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 prove an explicit transaction whose deferred commit trigger outlives client response observation remains unknown, later commits exactly one durable row, drains on the original session, and never replays. Connection-loss timing, TLS loss, rollback observation, reconnect ownership, reviewed plans, conflicts, service/history/UI/UniFFI remain open. |
| Phase 2 — PostgreSQL commit transport loss | [`165-phase-2-postgresql-commit-transport-loss.md`](../evidence/phase-2/postgres/165-phase-2-postgresql-commit-transport-loss.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 prove activity-gated server loss during deferred COMMIT yields unknown, terminal old sessions, explicit same-data-directory restart through a freshly resolved host mapping, observed rollback, and no replay. Pre/mid-dispatch loss, post-commit/pre-response loss, TLS loss, shared-service reconnect ownership, reviewed plans, conflicts, history/UI/UniFFI remain open. |
| Phase 2 — PostgreSQL mTLS commit loss | [`166-phase-2-postgresql-mtls-commit-loss.md`](../evidence/phase-2/postgres/166-phase-2-postgresql-mtls-commit-loss.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 prove active-COMMIT transport loss under verified custom roots and required client identity remains unknown, terminates old TLS sessions, rejects plaintext recovery, refreshes endpoint facts, revalidates mTLS, observes rollback, and never replays. Pre/mid-dispatch and post-commit/pre-response loss, identity rotation, shared-service reconnect ownership, reviewed plans, conflicts, history/UI/UniFFI remain open. |
| Phase 2 — PostgreSQL complex raw values | [`167-phase-2-postgresql-complex-raw-values.md`](../evidence/phase-2/postgres/167-phase-2-postgresql-complex-raw-values.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 preserve bounded JSON, JSONB, `int4range`, and anonymous-record binary payloads with exact type and truncation truth; large `bytea` remains binary. Research 168 promotes JSON/JSONB, research 180 ranges, and research 182 composites to `Structured`; strict pre-driver field allocation, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL JSON projection | [`168-phase-2-postgresql-json-projection.md`](../evidence/phase-2/postgres/168-phase-2-postgresql-json-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 project JSON/JSONB into deterministic compact bounded `Structured` values with arbitrary-precision number, malformed/version-invalid, and 8 MiB pre-DOM allocation truth. Raw inspector access, strict pre-driver field allocation, other structured decoders, service/UI, and UniFFI remain open. |
| Phase 2 — Redis atomic revocation fixture | [`170-phase-2-redis-atomic-revocation-fixture.md`](../evidence/phase-2/redis/170-phase-2-redis-atomic-revocation-fixture.md) | Reviewed 2026-07-17. Multi-user live revocation dispatches both administrative connection kills in one pipeline, removing stale reconnect interference while retaining independent server counts. This hardens evidence but closes no additional product capability. |
| Phase 2 — Redis subscription connect policy | [`171-phase-2-redis-subscription-connect-policy.md`](../evidence/phase-2/redis/171-phase-2-redis-subscription-connect-policy.md) | Reviewed 2026-07-17. Initial and replacement RESP2/RESP3 Pub/Sub generations share bounded cancellable connection attempts; required-TLS deadline exhaustion deterministically maps to `Connect`, plaintext blackholes remain `Timeout`, and initial setup emits no recovery gap. DNS change, strict pre-decode allocation, restricted denial, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL numeric decoder | [`172-phase-2-postgresql-numeric-decoder.md`](../evidence/phase-2/postgres/172-phase-2-postgresql-numeric-decoder.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 decode finite arbitrary-precision numeric values, declared scale/trailing zeros, scaled zero, NaN, and infinities into exact bounded `Decimal`; malformed and over-cell-limit values retain invalid/unknown truth. Other typed breadth, strict pre-driver allocation, service/UI, and UniFFI remain open. |
| Phase 2 — Redis administrative readiness budget | [`173-phase-2-redis-admin-readiness-budget.md`](../evidence/phase-2/redis/173-phase-2-redis-admin-readiness-budget.md) | Reviewed 2026-07-17. Raw TLS fixture administration overrides redis-rs's 500 ms response default with explicit bounded connection/response budgets and requires PING/PONG command readiness. This hardens the full real-server evidence but closes no additional product capability. |
| Phase 2 — PostgreSQL UUID decoder | [`174-phase-2-postgresql-uuid-decoder.md`](../evidence/phase-2/postgres/174-phase-2-postgresql-uuid-decoder.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 project representative, nil, and maximum UUID binary values into canonical lowercase bounded `Text`, with exact 36-byte truncation and malformed-length truth. Other typed breadth, service/UI, and UniFFI remain open. |
| Phase 2 — temporal value contract | [`175-phase-2-temporal-value-contract.md`](../evidence/phase-2/core/175-phase-2-temporal-value-contract.md) | Reviewed 2026-07-17. The shared owned-value and immutable-page contracts now carry a first-class bounded UTF-8 `Temporal` kind, including truncation validation and structured-projection support. Database decoders, canonical temporal forms, editors, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL temporal decoder | [`176-phase-2-postgresql-temporal-decoder.md`](../evidence/phase-2/postgres/176-phase-2-postgresql-temporal-decoder.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 binary date, time, timestamp, and timestamptz values now project into bounded canonical `Temporal`, preserving microseconds, 24:00, UTC instant truth, and infinities. Research 177 subsequently closes `timetz`, interval, and BC/expanded-year result decoding; editors, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL temporal completion | [`177-phase-2-postgresql-temporal-completion.md`](../evidence/phase-2/postgres/177-phase-2-postgresql-temporal-completion.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now project `timetz`, exact independent interval components and sentinels, astronomical BC dates, and signed expanded years into bounded canonical `Temporal`. Research 179/180 subsequently compose temporal values through arrays/ranges; editors, parameters/mutations, service/UI, and UniFFI remain open. |
| Phase 2 — ClickHouse temporal projection | [`178-phase-2-clickhouse-temporal-projection.md`](../evidence/phase-2/clickhouse/178-phase-2-clickhouse-temporal-projection.md) | Reviewed 2026-07-17. ClickHouse 25.8/26.3 LTS now project Date/Date32/DateTime/DateTime64 into bounded canonical `Temporal`, preserve declared fractional scale, normalize epoch instants to UTC, retain timezone metadata, and quote recursive temporal containers. Editors, mutation/input paths, UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL array projection | [`179-phase-2-postgresql-array-projection.md`](../evidence/phase-2/postgres/179-phase-2-postgresql-array-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now project generic binary arrays into bounded canonical `Structured` values while preserving dimensions, lower bounds, row-major nesting, NULL elements, and supported scalar kinds. Malformed structure becomes `Invalid`; valid unsupported or over-budget arrays remain whole-value `Unknown`. Research 182/183 subsequently close composite/domain elements; editors, public parameters, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL range projection | [`180-phase-2-postgresql-range-projection.md`](../evidence/phase-2/postgres/180-phase-2-postgresql-range-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now project generic binary ranges into bounded canonical `Structured` values with explicit empty, unbounded, inclusive, and exclusive truth across integer, numeric, date, and timestamp subtypes. Invalid flags/lengths become `Invalid`; unsupported subtype projection remains whole-value `Unknown`. Research 181 composes ranges through multiranges and research 182/183 close composite/domain decoding; editors, public parameters, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL multirange projection | [`181-phase-2-postgresql-multirange-projection.md`](../evidence/phase-2/postgres/181-phase-2-postgresql-multirange-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now project generic binary multiranges into ordered bounded canonical `Structured` members using the range contract, including empty, integer, unbounded, numeric, and date values. Invalid counts/lengths/member payloads become `Invalid`; over-budget or unsupported members remain whole-value `Unknown`. Research 182/183 subsequently close composite/domain decoding; editors, public parameters, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL composite projection | [`182-phase-2-postgresql-composite-projection.md`](../evidence/phase-2/postgres/182-phase-2-postgresql-composite-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now project named composites and anonymous records into bounded canonical `Structured` fields with names/null names, exact OID/type identity, NULL truth, nested arrays/ranges, strict framing, a 1,664-field ceiling, and a shared 64-level structured nesting cap. Unknown anonymous OIDs and over-budget structures remain whole-value `Unknown`; malformed values become `Invalid`. Research 183 subsequently closes domain decoding; editors, public parameters, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL domain projection | [`183-phase-2-postgresql-domain-projection.md`](../evidence/phase-2/postgres/183-phase-2-postgresql-domain-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now decode scalar, nested-domain, array, range, and composite domains through underlying semantic contracts while composite fields retain exact domain name/OID identity. Invalid/unknown fallback retains the outer domain identity and domains consume the shared recursion budget. PostgreSQL RowDescription flattens top-level domain expressions/columns to base types; this protocol truth is explicit. Editors, public parameters, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL enum projection | [`184-phase-2-postgresql-enum-projection.md`](../evidence/phase-2/postgres/184-phase-2-postgresql-enum-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now project ASCII and Unicode enum labels into bounded UTF-8-safe `Text` with exact user-defined column type identity. Invalid UTF-8 or labels absent from pinned catalog metadata become `Invalid` with enum identity. Editors/schema metadata, public parameters, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL network projection | [`185-phase-2-postgresql-network-projection.md`](../evidence/phase-2/postgres/185-phase-2-postgresql-network-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now project `inet`, `cidr`, `macaddr`, and `macaddr8` binary values into bounded canonical `Text` with exact type identity. Family, prefix, CIDR flag/network truth, address length, framing, and MAC width are strict; malformed values become `Invalid`. Editors/schema metadata, public parameters, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL bit-string projection | [`186-phase-2-postgresql-bit-string-projection.md`](../evidence/phase-2/postgres/186-phase-2-postgresql-bit-string-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now project fixed `bit` and variable `varbit` binary values into bounded canonical `Text` while retaining exact type identity. Signed count, exact payload size, trailing bytes, and unused padding bits are strict; malformed values become `Invalid`. Editors/type modifiers, public parameters, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL identifier projection | [`187-phase-2-postgresql-identifier-projection.md`](../evidence/phase-2/postgres/187-phase-2-postgresql-identifier-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now project `oid`, `xid`, `cid`, `xid8`, and all pinned `reg*` aliases into exact full-range core `Unsigned` values with column type identity. Wrong widths become `Invalid`; sub-eight-byte core bounds remain typed `Unknown`. Catalog names, tuple/vector/LSN/snapshot types, editors, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL LSN projection | [`188-phase-2-postgresql-lsn-projection.md`](../evidence/phase-2/postgres/188-phase-2-postgresql-lsn-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now project zero, representative, and maximum `pg_lsn` binary values into bounded canonical uppercase `HIGH/LOW` `Text` with exact type identity. Wrong widths become `Invalid`. Tuple/vector/snapshot types, editors, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL TID projection | [`189-phase-2-postgresql-tid-projection.md`](../evidence/phase-2/postgres/189-phase-2-postgresql-tid-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now project first, maximum, and live `tid` values into bounded structured unsigned block/offset pairs with strict six-byte framing. `tid` remains physical row-version evidence and is explicitly forbidden as durable identity or automatic mutation locator. OID vectors/snapshots, stable locators, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL OID-vector projection | [`190-phase-2-postgresql-oid-vector-projection.md`](../evidence/phase-2/postgres/190-phase-2-postgresql-oid-vector-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now project representative, empty, and unsigned-boundary `oidvector` values into bounded ordered structured OID lists. One dimension, zero lower bound, no NULLs, OID element identity, exact member/framing, and one-million count ceiling are strict. Snapshots, catalog interpretation/editing, service/UI, and UniFFI remain open. |
| Phase 2 — PostgreSQL snapshot projection | [`191-phase-2-postgresql-snapshot-projection.md`](../evidence/phase-2/postgres/191-phase-2-postgresql-snapshot-projection.md) | Reviewed 2026-07-17. PostgreSQL 17.10/18.4 now project modern `pg_snapshot` and legacy `txid_snapshot` into one bounded structured xmin/xmax/in-progress shape while retaining exact column identity. Nonzero ordered bounds, strictly increasing in-range members, exact framing, and a one-million count ceiling are strict. Catalog interpretation/editing, service/UI, and UniFFI remain open. |
