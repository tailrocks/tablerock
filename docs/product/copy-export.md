# Copy And Export

Every grid — table browsing or query results — copies and exports what it
shows.

## Copy formats

Selection scope: current cell, selected cells, selected rows, or the whole
loaded result. Formats:

| Format | Shape |
|---|---|
| CSV | header row, RFC-4180 quoting |
| TSV | clipboard-friendly, no quoting |
| JSON | array of objects, typed values where representable |
| SQL INSERT | one statement per row, fully qualified table, typed literals |
| SQL UPDATE | one statement per row keyed by stable identity |
| Markdown | pipe table for docs/chat |

INSERT/UPDATE copies require the same editability facts as editing: a known
base table and stable row identity. Where identity is missing, those two
formats are absent — not degraded to broken statements.

Binary, NULL, truncated, and unknown values copy with explicit
representations; a truncated value is marked, never silently shipped as
complete.

## Export

- Export the loaded result or a streaming full re-query to CSV or JSON
  files with progress and cancellation.
- Destinations are atomic: partial files from cancelled or failed exports
  are removed.
- SQL-form export (INSERT dumps) ships with the import/export phase, using
  bounded streaming.

## Import

CSV/JSON import into a chosen table arrives in the data-movement phase:
column mapping, encoding handling, progress, cancellation, and explicit
partial-import outcomes. Formula-like cell content is treated as data, never
evaluated.

Reviewed CSV application is a Rust-owned asynchronous operation. The native
sheet polls bounded row progress, can request cancellation, distinguishes
PostgreSQL rollback from ClickHouse partial apply, retains at most 100 safe
row-number errors, and can copy that error summary. Closing or disconnecting
cannot abandon a running import. Preview scans up to 16 GiB/100 million rows
with fixed memory and returns a SHA-256 fingerprint. Review rejects file drift,
copies accepted bytes into a private frozen spool, validates every typed value,
then consumes one authority into 500-row/8 MiB batches. PostgreSQL commits each
batch transactionally; ClickHouse reports progressive partial truth. Frozen
files are removed on reject, expiry, discard, terminal outcome, or teardown.

## Both clients

| | TUI | Native macOS |
|---|---|---|
| Copy | clipboard adapter effect, format picker | pasteboard with multiple representations |
| Export | file path dialog, progress in status bar | `NSSavePanel`, progress indicator |
| Import | file picker, mapping screen | `NSOpenPanel`, mapping sheet |

Format generation is Rust-owned in both clients; presentation only selects
scope and format.
