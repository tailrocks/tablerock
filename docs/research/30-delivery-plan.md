# Delivery Plan

This is a multi-PR program. Research does not justify wiring a partial UI or
adding all dependencies at once.

## Invariants

1. Only PostgreSQL, ClickHouse, and Redis in the first program.
2. Concepts-only reference use; no copied source/expression.
3. Database I/O outside TUI update/render.
4. Rust authoritative state; presentation focus/layout.
5. Batches/pages across boundaries, never per-cell calls.
6. Secrets, SQL, and cell values redacted from default telemetry.
7. Safety enforced below presentation.
8. Focus/actions/progress/cancellation/truncation visible.
9. Schema/protocol changes include version, migration, fixtures, docs.
10. Every phase leaves honest buildable behavior.

## Phase 0: Contracts and spikes

Deliver:

- `tablerock-core` IDs/capabilities/values/pages/results/errors;
- driver contract harness with pinned database containers;
- PostgreSQL streaming/dynamic types/prepared/TLS/cancel spike;
- official ClickHouse self-describing stream/partial error/progress/query ID/TLS/cancel spike;
- `redis-rs` bytes/SCAN/logical DB/TLS/reconnect/timeout/cancel spike;
- typed bounded Redis INFO spike;
- SQL parser/incomplete token fallback and editor/focus spike;
- embedded storage and license-policy decisions;
- first-row/throughput/cancel/allocation/memory measurements.

Accept when overlapping contracts pass, differences are capabilities, arbitrary
ClickHouse results use the official client without compile-time row structs,
Redis is binary-safe/SCAN-based, and rejected dependencies are documented.

## Phase 1: Profiles and connection shell

Deliver connection list, three-engine picker, General/TLS/Safety forms,
metadata-only 1Password picker/mapping, canonical per-field OpRefs, prompt/env/
dangerous-plaintext alternatives, Test, saved/temporary connect, and versioned
config.

Accept when one item maps all connection/TLS fields without picker resolution;
Test/Connect resolves mapped fields only; plaintext remains acknowledged/visible;
secrets never enter config/snapshots/logs; all engines pass local and verified
TLS fixtures; no `jackin❯` internal dependency exists.

## Phase 2: PostgreSQL read-only slice

Deliver sessions/results, connection state, incremental databases/schemas/
tables/views/structure, table pages, query tabs, streaming/cancel/status/errors,
typed values/unknown fallback, read-only enforcement, and redacted telemetry.

Accept when first rows render before completion, database/schema semantics are
correct, server cancellation is proven, caps are exact, stale events cannot
cross reconnect/context revisions, and unknown values are non-editable.

## Phase 3: Grid, editor, completion, and PostgreSQL editing

Deliver viewport grid, stable two-axis scroll, typed display states, bounded
prefetch; multiline editor, syntax, diagnostics, revisioned autocomplete,
results split; typed cell inspector/editors; mutation reducer/review;
parameterized PostgreSQL changes, transaction apply, conflicts, rollback, and
generated-value reconciliation.

Accept when resident scrolling performs no I/O/full allocation; stale pages and
completion reject by revision; minimum/normal/wide renders do not overlap;
hostile names/values cannot alter SQL structure; tab changes are all-or-rollback;
joins/aggregates/no-key results are read-only; ambiguous writes never retry.

## Phase 4: ClickHouse slice

Deliver official-driver databases/objects/structure/DDL, table pages, arbitrary
dynamic SQL streaming, progress/query ID/cancel/partial states, complex values,
batch inserts, Parts/mutation status, and gated UPDATE/DELETE.

Accept when the official driver is the transport absent an approved upstream
gap; dynamic results need no Rust row struct; client stop/server cancel differ;
partial errors stay honest; HTTPS passes; mutations remain visible and are never
labeled transactional.

## Phase 5: Redis slice

Deliver isolated logical DB selection, SCAN cursors/namespaces/filter/refresh,
typed bounded values/TTL, binary inspection, current INFO Overview, command
editor/completion/results/cancel, staged type edits, command/TTL review, and
destructive/unknown-command gating.

Accept when browsing never uses KEYS; changing keyspaces remain safe without
false totals; bytes round-trip; large values are bounded; logical DBs cannot
race; blocking commands are denied/isolated; MULTI/EXEC is not rollback;
Overview tolerates denied fields and stores no raw response/history.

## Phase 6: Daily-use hardening

Deliver history/search, intent restoration, health/reconnect/backoff/shutdown,
result eviction/budgets, support/version matrix, performance thresholds,
complete docs, OpenTelemetry, and clean-room/license provenance audit.

Accept when relaunch restores without reconnect storms or persisted results/
edits; memory meets measured budgets; one failed tab does not block the app;
every destructive path has safety tests; support claims match CI exactly.

## Phase 7: Daemon and native macOS

First deliver versioned local commands/events/encoded pages,
`tablerock-daemon` session authority, peer authorization, bounded subscriptions,
cancel/version/redaction/restart/idempotency, and TUI adapter parity.

Then deliver native SwiftUI/AppKit connection/catalog/tab/grid/editor/review/
settings/accessibility UI, daemon client, explicit embedded Rust decision, and
VoiceOver/keyboard/appearance/multi-window/Swift 6/performance tests.

Accept when one live-session authority exists; slow clients cannot exhaust
buffers; restart cannot repeat ambiguous writes; no per-cell boundary exists;
and Swift contains no database driver/core duplication.

## Later programs

- import/export, backup/restore, schema/DDL editing, visual plans;
- advanced PostgreSQL/ClickHouse/Redis administration and cluster breadth;
- query files/file browser;
- SSH/cloud tunnels and cloud identity;
- AI, MCP, or agent database access under separate least-privilege, approval,
  audit, prompt-injection, and destructive-operation research.

## First-program definition of done

- all three engines support profile/test/connect/catalog/browse/query-or-command/cancel/TLS;
- PostgreSQL editing is transactional/conflict-aware;
- ClickHouse/Redis expose honest engine-specific writes;
- results are bounded, streaming, cancellable, and redacted;
- 1Password is preferred and plaintext remains explicitly dangerous;
- terminal conformance and measured performance gates pass;
- history/restoration/safety survive relaunch/failure;
- docs state tested versions/limitations;
- source/license provenance passes independent review.
