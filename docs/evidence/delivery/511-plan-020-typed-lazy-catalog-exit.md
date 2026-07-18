# Plan 020 — typed lazy catalog and Phase 13 exit

Date: 2026-07-19

## Structural correction

The first native catalog bridge used Rust-owned SQL behind a generic operation,
then returned a flat result page. That kept SQL out of Swift but bypassed the
existing engine-native `CatalogRequest` hierarchy and could not perform honest
lazy expansion.

The UniFFI facade now exposes one coarse typed `refresh_catalog` operation. It
returns bounded Rust-owned node facts and opaque node IDs. Swift may return only
an ID previously issued for that session; Rust caches the node kind/ancestry,
chooses the engine request, rejects unknown/stale handles, validates returned
child kinds, bounds node/text totals, and allocates new opaque IDs. Root refresh
invalidates every old handle; subtree refresh invalidates all old descendant
handles. The obsolete SQL-backed catalog intent was removed rather than kept as
a parallel path.

Live testing exposed an existing service-layer mismatch: a child-only subtree
was wrapped as a full `CatalogSnapshot`, whose correct preorder validator
rejected the absent parent with `ParentNotBeforeChild`. The bridge now consumes
the already bounded typed `CatalogSubtree` directly instead of weakening full
snapshot validation. `EngineServiceError` retains the concrete catalog build
error so future failures are diagnosable.

## Native behavior

`NSOutlineView` now renders the Rust hierarchy directly. Expansion dispatches
the opaque parent ID, shows `Loading…` under that node, preserves the old
subtree on failure as `Stale · <safe error>`, and restores expansion/selection
without redispatch loops. Initial loading and failure remain explicit screens.
The runtime AppKit fixture proves expansion dispatch plus loading-to-stale state
projection.

## Evidence

| Gate | Result |
|---|---|
| PostgreSQL 18.4 typed traversal | pass; database → schema → relations, 86 nodes |
| ClickHouse 25.8 typed traversal | pass; database → objects, 25 nodes |
| Redis 8.0 typed root | pass; 16 logical databases; key browser remains Plan 021 |
| PostgreSQL query/cancel/review | pass; cancellation 0.190 s, reviewed apply committed |
| `cargo test -p tablerock-ffi` | 17 passed, 5 real-server tests ignored |
| typed catalog conformance | opaque-parent traversal + stale-handle rejection pass for all engines |
| engine-service tests | 7 passed |
| native strict build | pass |
| native runtime accessibility/catalog-state gate | pass |

This closes the Plan 020 vertical slice and ROADMAP Phase 13. Full catalog
filtering, Redis key namespaces, object opening/tabs, and the complete system
accessibility matrix remain visible Plan 021 parity work.

## Provenance

TablePro was used only to confirm the broad concept of a lazily expanding
database object browser. No source, tests, text, screenshots, layouts,
measurements, colors, assets, or key bindings were copied or translated.
