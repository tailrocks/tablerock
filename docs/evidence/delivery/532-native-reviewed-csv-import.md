# Native reviewed CSV import

Date: 2026-07-19

## Native workflow

Writable PostgreSQL and ClickHouse object tabs expose `Import CSV`. The native
open panel grants balanced security-scoped access, Rust scans at most 16 GiB
and 100 million rows with fixed memory, and the sheet previews up to 100 rows.
Operators map
every CSV header to a unique target column and explicitly choose Text, Integer,
Float, or Boolean value typing.

Staging re-reads the file, parses mapped values into typed
`MutationChange::InsertRow` facts, resolves the target only from the live
session's cached catalog node, and freezes the plan behind a 60-second
consume-once review token. The sheet states row/column count, target, literal
formula handling, expiry, and non-retry authority before Apply. Dismissal is
blocked while authority exists; discard revokes it. Apply consumes authority
before I/O and refreshes the object after an unconflicted outcome.

## Safety corrections

Live PostgreSQL testing proved the prior all-text CSV projection could not write
numeric columns. Silent inference was rejected. Shared conversion now requires
an explicit value type per mapped column and returns row/column diagnostics for
invalid integers, floats, and booleans before review creation.

The registered Rust session now retains its bounded database identity. Callers
cannot supply a schema, table, database, or mutation plan through UniFFI.

## Evidence

- `tablerock-files`: 17 tests pass, including bounded file reads, invalid UTF-8,
  explicit typed conversion, malformed grammar, and formula-literal behavior.
- FFI: full suite 20 tests passes, 5 live tests ignored; deterministic
  PostgreSQL and ClickHouse catalog-table staging, mapping, preview, and token
  revocation pass.
- Full CLI suite: pass.
- `tablerock-files` and FFI Rust 1.97 clippy with warnings denied: pass.
- Live PostgreSQL 18.4 native gate: two mapped rows apply in one transaction,
  server count is two, `=literal` remains exact text, preview/mapping sheet is
  visible, and refreshed object rows show both values: pass.
- Native object-tab and accessibility structural/runtime regressions: pass.

## Remaining boundary

Nullable/date/decimal/binary/structured mappings and JSON import remain.
Evidence 644–646 adds fingerprint-bound frozen streaming, Rust-owned progress/
cancellation, live ClickHouse apply, and explicit partial-failure outcomes.

## Provenance

TablePro was used only to confirm the broad preview, mapping, and review
workflow. No source, tests, text, screenshots, layouts, measurements, colors,
assets, or key bindings were copied or translated.
