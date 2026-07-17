# Phase 2 ClickHouse Complex Scalars

## Decision

TableRock extends its sole `RowBinaryWithNamesAndTypes` decoder. It does not add
a JSON, CSV, typed-row, or hand-written HTTP alternative for arbitrary results.
Exact ClickHouse type strings remain in column metadata.

The decoder now covers:

- `Bool`, every signed and unsigned width from 8 through 256 bits;
- `Float32` and `Float64`, preserving the projected IEEE 754 payload;
- `Decimal32`, `Decimal64`, `Decimal128`, `Decimal256`, and precision-based
  `Decimal(P,S)`;
- canonical bounded `Date`, `Date32`, `DateTime`, and timezone-bearing
  `DateTime64` temporal projections (completed by research 178);
- `UUID`, `IPv4`, `IPv6`, `Enum8`, `Enum16`, and `LowCardinality(T)`;
- the existing strings, fixed strings, binary strings, and recursive nullable
  scalars.

Values wider than TableRock's native 64-bit integer slots are converted from
little-endian two's-complement bytes into exact bounded base-10 text and stored
as arbitrary-precision decimal values. No float conversion occurs. Decimal
scale placement is exact, including negative values and values between -1 and
1. If the exact text exceeds the cell budget, the value becomes a bounded
`Unknown` carrying its exact ClickHouse type and raw prefix; truncation records
the original byte length. This preserves inspectability without lying about
precision.

## Bounds and failure

- Fixed widths are validated before allocation and never exceed 32 bytes.
- Decimal precision is accepted only in ClickHouse's 1–76 range.
- Invalid bool/null markers, widths, scales, type syntax, or truncated payloads
  fail closed.
- Tight cell budgets do not desynchronize later columns or rows.
- Adapter errors and diagnostics contain no SQL text or cell values.

## Evidence

The digest-pinned ClickHouse 25.8.28.1 and 26.3.17.4 LTS Testcontainers matrix
runs every probe with compression disabled and LZ4 enabled. It proves exact
UInt128/Int128/UInt256/Int256 extrema, Decimal256 scale, Float32 projection,
canonical temporal values/metadata, UUID/IP raw values, enums, low-cardinality text, and bounded
unknown/truncation warnings. Unit tests independently prove full-width
two's-complement conversion and decimal scale placement around zero.

Arrays, tuples, maps, nested values, variants/dynamic values, aggregate-state
fallback, TLS fixtures, progress, cancellation outcome, late errors, inserts,
and mutations remain Phase 2 blockers.

External concepts: ClickHouse RowBinary scalar encoding, two's-complement conversion, fixed-point scale
Public sources: <https://github.com/ClickHouse/clickhouse-rs/tree/v0.15.1>, <https://clickhouse.com/docs/interfaces/formats/RowBinary>, <https://clickhouse.com/docs/sql-reference/data-types>
Implementation source: official ClickHouse client source/tests, primary format/type docs, and TableRock-owned decoder/tests
Copied code/assets/text: none
