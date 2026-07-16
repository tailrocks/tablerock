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
| Profile organization | Parity | Groups, tags, favorites, ordering, filters, and environment/safety markers without copied reference presentation | Phase 2 proves profile contracts plus a local-only serialized Turso actor, normalized process-local single ownership, interrupted/crash recovery, sequential profile migrations, atomic CRUD, and a least-data keyset-paginated organization list capped at 100 with cursor-bound engine/favorite/group/tag filters; normalized search, endpoint/health projection, resolution, UI, unrelated-entity retention, and remaining storage proof stay required |
| Context switcher | Core | Switch connection/database/schema/logical database using engine-correct semantics | Driver contract tests prove no fake cross-engine abstraction |
| Health and reconnect | Core | Visible state, bounded backoff, authentication stop condition, context restoration, and no automatic ambiguous-write retry | Disconnect/reconnect race harness |
| Startup actions | Later | Reviewed, bounded startup SQL/commands with explicit reconnect behavior | Safety classification, timeout, and partial-failure tests |
| SSH transport | Parity | Rust `russh` adapter below clients with host-key verification, known-hosts, key/agent/password auth, keepalive, cancel, and reconnect | Real SSH bastion matrix; no shell interpolation or secret logging |
| Cloud-provider proxy/identity | Excluded | No vendor proxy/identity workflow in this three-engine program | Product-boundary test and documentation |

## Workbench and navigation

| Capability | Status | TableRock requirement | Acceptance evidence |
|---|---|---|---|
| Lazy catalog | Core | Engine-native hierarchy, loading/stale/error nodes, subtree refresh, filtering with ancestors preserved | Large synthetic catalogs and stale-revision tests |
| Object tabs | Core | Preview/pinned/durable object tabs with independent context and state | Restore and close-policy tests |
| Query/command tabs | Core | Independent text, cursor, context, results, errors, history, and running operation | Phase 2 now proves typed command scopes, finite budgets, scoped operation identity, legal lifecycle/cancellation-outcome edges, stale/duplicate rejection, and gap resync; execution text, multi-tab engine behavior, and cancellation races remain required |
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
| Typed display | Core | Distinguish NULL, empty, whitespace, zero, false, bytes, unknown, invalid, and truncated values | The Phase 2 owned-value unit contract covers null/empty/whitespace/zero/false/bytes/unknown/invalid/truncated distinctions; cross-engine value corpus remains required |
| Column controls | Parity | Width, fit, hide, order, format, and stable per-object preferences | Narrow/wide/Unicode geometry tests |
| Sorting | Core | Server sort for table browsing, explicit provenance, multi-column order, no unsafe SQL concatenation | Hostile identifier/type fixtures |
| Filtering | Core | Typed server filters plus clearly labeled resident-page value filters; saved presets later | Parameterization and NULL semantics tests |
| Selection/copy | Parity | Cell/range/row selection and safe TSV/CSV/JSON/Markdown projections | Clipboard-neutral formatter tests |
| Paging | Core | Bounded server pages for objects and bounded streams for arbitrary queries; totals may be estimated/unknown | The Phase 2 immutable-page unit contract covers pre-allocation dimensions, ranges, offsets, null/truncation metadata, unknown totals, and hostile buffers; page races, eviction, count budgets, bridge encoding, and real-server evidence remain required |
| Row/value inspector | Core | Full typed values, raw bytes/hex, JSON text/tree, metadata, and stale state | Large/binary/invalid JSON fixtures |
| Type-specific editors | Parity | Bool, number, temporal, enum, JSON, bytes, array/map/tuple, and explicit unknown fallback | Round-trip and invalid-input tests |
| Stable editability | Core | Only results with proven base object and stable identity are editable | Joins/aggregates/no-key/duplicate-key tests |
| Staged changes | Core | Inserts/updates/deletes remain local, visible, undoable, reviewable, and discardable until apply | Reducer model tests and quit/refresh policy tests |
| Operation preview | Core | Preview is descriptive; execution uses a typed parameter/command plan, never reparsed display text | Mutation-plan serialization tests |
| Conflict handling | Core | PostgreSQL conflicts roll back; ClickHouse and Redis expose their true non-transactional outcomes | Concurrent-change integration tests |
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
