# Native column-layout bridge foundation

Date: 2026-07-22

## Outcome

The shared Rust bridge now exposes typed load/save/reset operations for a
table-like opaque catalog target. Rust derives profile/database/schema/table
ownership from the live session and cached catalog; Swift cannot forge the
persistence key. Layouts are bounded to 512 unique columns, keep at least one
visible column, use shared 4–80 character-cell widths, and never contain cells
or result values. PostgreSQL and ClickHouse table-like objects are supported;
other targets fail closed.

Native presentation controls and AppKit source-index repair remain open, so
`TR-SCR-015` remains partial.

## Verification

```text
mise exec -- cargo test -p tablerock-ffi --test conformance \
  catalog_column_layout_is_typed_bounded_and_persisted_by_opaque_target --locked
mise exec -- cargo clippy -p tablerock-ffi --all-targets --locked -- -D warnings
mise exec -- bash scripts/generate-swift-bindings.sh
```

## Clean-room provenance

TablePro public data-grid documentation established only the broad workflow:
visibility, resizing, auto-fit, and per-table persistence. No source, tests,
identifiers, strings, assets, screenshots, measurements, colors, or key
bindings were copied. The contract derives from TableRock requirements and its
existing Turso persistence actor.
