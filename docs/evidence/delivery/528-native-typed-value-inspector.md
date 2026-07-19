# Native typed value inspector

Date: 2026-07-19

## Typed page projection

Swift's bounded page-v1 decoder now retains Rust-owned column engine type,
nullability, value kind, truncation tag/original byte count, and raw bytes.
Full decode validates magic, version, and allocation bounds before body
allocation. Display rows remain immutable convenience projections over matching
typed cells.

## Native behavior

AppKit grid clicks identify row plus clicked column. Selection stays owned by
its query or object tab. A native split inspector shows row/column position,
column/type/nullability facts, value kind, stored byte count, explicit
truncation warning, selectable text, and hexadecimal bytes. Invalid selection
after page replacement fails closed by hiding the inspector.

## Evidence

- Typed inspector structural/runtime fixture: pass.
- Real PostgreSQL, ClickHouse, and Redis page decode with metadata/cell shape
  invariants: pass.
- Native page decode/scroll/leak performance gate: pass; 500 rows, 2 columns,
  2,000 decodes, 0 leaks.
- Query-tab, object-tab, and accessibility regressions: pass.

## Remaining boundary

Native structured JSON tree, binary-specific layouts, stale-page marker, and
editable typed controls remain. This checkpoint claims read-only text/hex/fact
inspection only.

## Provenance

TablePro was used only to confirm the broad value-inspector workflow. No source,
tests, text, screenshots, layouts, measurements, colors, assets, or key bindings
were copied or translated.
