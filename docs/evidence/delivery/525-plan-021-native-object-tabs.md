# Plan 021 — native object tabs

Date: 2026-07-19

## Rust browse authority

UniFFI accepts a live session, opaque cached catalog-node handle, and bounded
row count. Rust verifies session/node ownership, rejects stale handles and
non-table-like kinds, resolves the parent schema/database, and renders the
existing typed `BrowsePlan`. Row counts must be 1–1,000. Swift never receives or
constructs browse SQL.

PostgreSQL tables, views, materialized views, foreign/partitioned tables, and
sequences are supported. ClickHouse tables, views, materialized views, and
dictionaries are supported. PostgreSQL functions/types and Redis logical nodes
reject explicitly.

## Native behavior

Double-clicking a browsable catalog node opens a read-only preview object tab.
Leaving the preview for another tab pins it; Pin is also explicit. The same
opaque object may open multiple times, each with independent decoded result,
result ID/revision, page cursor, running operation, summary, and error state.
Tabs support refresh, cancellation through the shared toolbar, bounded load
more, and guarded close while running.

Connection replacement is blocked while any query or object tab runs. On a
successful replacement all object handles and volatile object state are
discarded because catalog handles are session-scoped.

## Evidence

| Gate | Result |
|---|---|
| cached table-like node browse conformance | pass; PG + ClickHouse |
| root/non-object and zero-row-bound rejection | pass |
| UniFFI suite | pass; 20 tests, 5 ignored |
| native duplicate-object/result-isolation fixture | pass |
| preview-to-pin and close/running structural gate | pass |
| live PostgreSQL `public.users` opaque browse/decode | pass; 1 row |
| live ClickHouse `db.events` opaque browse/decode | pass; 1 row |
| live Redis baseline | pass; object browse not applicable |
| native query-tab/accessibility regressions | pass |

## Remaining boundary

Per-object sort/filter/column layout, staged changes, structure/function
inspectors, durable object-tab restoration, and Redis key object tabs remain.
This checkpoint does not claim them.

## Provenance

TablePro was used only to confirm broad object preview, pin, and duplicate-tab
concepts. No source, tests, text, screenshots, layouts, measurements, colors,
assets, or key bindings were copied or translated.
