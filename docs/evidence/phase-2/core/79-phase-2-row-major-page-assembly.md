# Phase 2 Row-Major Page Assembly Evidence

## Checkpoint

Database adapters can now convert bounded owned row-major values into the sole
immutable `ResultPage` representation through `ResultPage::from_row_major`.
Driver rows and client types never cross the adapter boundary.

## Contract

The assembler accepts core identities, page facts, column metadata, owned
values, and explicit `PageLimits`. Before allocating the columnar arena it
rejects excessive columns, rows, cell-shape mismatches, arena bytes, column
metadata bytes, range overflow, and known totals before the page end.

Accepted input is transposed into the canonical column-major layout. It
preserves NULL, value kind, truncation metadata, big-endian fixed-width values,
and exact text/binary/unknown bytes. The existing page validator rechecks the
finished offsets, bitmap, encodings, nullability, engine identity, and declared
lengths. There is no driver-specific page representation or compatibility path.

## Failure and safety

- Ragged row input fails with `CellCountMismatch`; it is never padded.
- Limits fail before output arena allocation.
- Debug output remains metadata-only because `OwnedValue`, `PageBuffers`, and
  `ResultPage` do not render cell payloads.
- Cancellation remains an engine-operation concern. Assembly is synchronous,
  finite, and receives only a completed bounded batch.

## Evidence

- A two-row page proves row-to-column transposition, signed big-endian encoding,
  text preservation, and nullable NULL projection.
- Ragged cells and an eight-byte value against a seven-byte arena limit fail.
- All hostile buffer and envelope validation fixtures remain green.

## Verification record

- `cargo test -p tablerock-core --test page`: 8 passed.
- `cargo test --workspace --all-targets --locked`: 107 passed, 3 ignored.
- Workspace format, Clippy with warnings denied, rustdoc, diff, English-only,
  redaction, architecture, and provenance review: pass.

## Deliberate boundary

This closes the shared adapter-to-core assembly prerequisite only. PostgreSQL,
ClickHouse, and Redis must independently prove bounded streaming, value mapping,
cancellation, and real-server behavior before any driver support claim.

External concepts: bounded batch-to-columnar result assembly
Public sources: PostgreSQL protocol format and message-boundary documentation; tokio-postgres 0.7.18 streaming documentation
Implementation source: TableRock-owned page contract and tests
Copied code/assets/text: none
