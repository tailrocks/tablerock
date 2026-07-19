# Shared result copy and native pasteboard

Date: 2026-07-19

## Shared Rust ownership

Copy formatting moved from TUI-only string inference into `tablerock-core`.
Both clients now adapt explicit typed cells into one bounded formatter for CSV,
TSV, JSON, Markdown, SQL INSERT, and SQL UPDATE. NULL, boolean, number, binary,
unknown, invalid, and truncated values remain explicit. Output is capped at
10,000 rows, 1,024 columns, and 16 MiB.

SQL INSERT requires Rust-cached base-table identity from opaque catalog browse.
SQL UPDATE additionally requires proven stable identity columns and otherwise
fails closed; it no longer emits a dangerous commented placeholder WHERE.

## Native behavior

Cell, row, and loaded-result scopes call UniFFI with only result handle,
revision, and selection coordinates. Rust reads resident pages and returns
formatted payloads. Swift writes one `NSPasteboardItem` containing plain text,
CSV, TSV, JSON, and Markdown representations. SQL INSERT appears only for
supported PostgreSQL and ClickHouse base-table object tabs. Pagination appends
typed cell metadata alongside display rows.

## Evidence

- Core formatter: all six formats, typed edge cases, bounds, stable-identity
  rejection, and ordered current-revision resident-page access: pass; full core
  suite 150 tests pass.
- TUI shared-formatter adapter: 10 tests pass; full TUI suite 315 tests pass.
- UniFFI opaque result formatting and table-identity gate: pass; full suite 20
  tests, 5 ignored.
- Live PostgreSQL native model/pasteboard: five representations, typed JSON,
  all three scopes wired: pass.
- Query tabs, object tabs, inspector, accessibility regressions: pass.
- Core/FFI Rust 1.97 clippy with warnings denied: pass. Workspace TUI clippy
  still has pre-existing repository-wide warnings outside this checkpoint.

## Remaining boundary

Native file export/import, selected multi-cell ranges, stable-identity SQL
UPDATE, and full streaming export remain. This checkpoint claims clipboard
copy for cell, row, and loaded-result scopes plus identity-safe SQL INSERT.

## Provenance

TablePro was used only to confirm broad multi-format copy and export workflows.
No source, tests, text, screenshots, layouts, measurements, colors, assets, or
key bindings were copied or translated.
