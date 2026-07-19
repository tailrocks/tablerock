# Native ClickHouse structure facts

Date: 2026-07-19

## Shared ownership

`tablerock-engine::load_relation_structure` now supports ClickHouse through
engine-specific adapter methods. Named ClickHouse parameters select cached
catalog database/table identity; presentation cannot inject metadata SQL.
`system.columns` supplies ordered type, default, comment, primary-key, and
sorting-key facts. `system.tables` supplies engine, partition, sorting,
primary-key, and create-query facts. Fetching is bounded to 512 columns and
4 MiB per metadata response.

PostgreSQL indexes/constraints remain PostgreSQL facts. ClickHouse does not
invent equivalent rows. TUI text formatting and native UniFFI both adapt the
same typed snapshot.

## Native behavior

Existing per-object Data/Structure state renders ClickHouse columns plus a
selectable Engine facts section. Empty defaults, comments, and engine
expressions remain explicit. Primary and sorting membership are distinct.

## Evidence

- Live ClickHouse 25.8 native gate: three-column `MergeTree` table; `UInt64`
  primary/sorting column, nullable type, `now()` default, comment,
  `toYYYYMM(created_at)` partition key, engine, and create query all cross the
  shared Rust/UniFFI/Swift path; visible native screen audit passes.
- Live PostgreSQL 18.4 native structure regression: pass.
- Engine, TUI structure, CLI check, and FFI conformance regressions: pass.
- Native direct Swift 6 strict-concurrency build: pass.

## Remaining boundary

Native copied DDL, richer PostgreSQL column metadata, durable object-tab
restoration, and reviewed structure editing remain.

## Provenance

TablePro was used only to confirm broad object-structure workflow. No source,
tests, text, screenshots, layouts, measurements, colors, assets, or key
bindings were copied or translated.
