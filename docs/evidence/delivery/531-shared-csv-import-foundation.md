# Shared bounded CSV import foundation

Date: 2026-07-19

## Structural ownership

CSV parsing and typed insert-change projection moved from the CLI adapter into
`tablerock-files`. TUI and native adapters can now consume one parser without a
presentation dependency or SQL-string construction. Existing CLI reviewed
apply behavior remains on the same shared implementation.

## Hardening

- At most 1,024 columns, caller-bounded rows, and caller-bounded cell bytes.
- Quotes are accepted only at field start; content after a closing quote and
  quotes inside unquoted fields fail with row/column diagnostics.
- Empty and duplicate headers fail before mutation planning.
- Empty quoted fields remain data.
- `=`, `+`, `-`, and `@` spreadsheet-formula prefixes are identified after
  leading whitespace but remain literal text and are never evaluated.
- Insert conversion produces typed `MutationChange::InsertRow` values only.

## Evidence

- `tablerock-files`: 15 tests pass, including malformed quotes, header
  ambiguity, width bounds, formula detection, atomic cleanup, and concurrent
  writer isolation.
- Full CLI suite: pass.
- `tablerock-files` Rust 1.97 clippy with warnings denied: pass.

## Remaining boundary

Native file selection, preview/mapping, explicit review, transactional apply,
JSON import and broader type conversion remain. Evidence 644–646 supersedes
the former streaming/progress/cancellation residual with frozen-file batches
and live PostgreSQL/ClickHouse outcomes.
This foundation alone does not claim a native import screen.

## Provenance

TablePro was used only to confirm the broad import-preview workflow. No source,
tests, text, screenshots, layouts, measurements, colors, assets, or key
bindings were copied or translated.
