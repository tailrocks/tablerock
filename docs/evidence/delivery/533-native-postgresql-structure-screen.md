# Native PostgreSQL structure screen

Date: 2026-07-19

## Shared ownership

PostgreSQL structure catalog SQL moved out of the CLI presentation adapter into
`tablerock-engine::load_relation_structure`. The bounded typed snapshot owns
columns, nullability, defaults, indexes, and constraints. TUI formatting now
adapts that snapshot, and native UniFFI projects the same facts from an opaque
session plus cached catalog-node handle. Swift cannot provide schema/table
strings or catalog SQL.

The loader caps columns at 256, indexes and constraints at 128 each, cells at
8 KiB, and each section at 256 KiB. Non-PostgreSQL objects return an explicit
unavailable result; no cross-engine structure fiction is presented.

## Native behavior

Each object tab independently retains Data/Structure selection, loading,
snapshot, and error state. Structure renders selectable Columns, Indexes, and
Constraints sections with explicit empty and unavailable states. Refreshing or
switching another object cannot overwrite the tab's structure state.

## Evidence

- Full engine suite: pass.
- Full CLI suite: 41 passed, 7 ignored; migrated TUI structure behavior remains
  on the shared typed snapshot.
- Full TUI suite: 315 passed.
- Full FFI suite: 20 passed, 5 ignored; ClickHouse explicit rejection is
  covered by conformance, while PostgreSQL opaque-target success is live-proven.
- Live PostgreSQL 18.4 native gate: three columns preserve type, nullability,
  and `now()` default; primary and secondary indexes plus named CHECK
  constraint are present; visible native screen audit passes.
- Native object-tab and accessibility structural/runtime regressions: pass.
- FFI clippy is clean. Full engine clippy reaches 11 existing Rust 1.97
  warnings outside this checkpoint; no new warning originates in the structure
  module.

## Remaining boundary

ClickHouse structure/engine facts, PostgreSQL richer column metadata, copied
DDL from the typed snapshot, durable object-tab restoration, and reviewed
structure editing remain.

## Provenance

TablePro was used only to confirm the broad object-structure workflow. No
source, tests, text, screenshots, layouts, measurements, colors, assets, or key
bindings were copied or translated.
