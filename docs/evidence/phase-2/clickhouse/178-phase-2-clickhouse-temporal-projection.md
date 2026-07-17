# Phase 2 ClickHouse Temporal Projection Evidence

## Checkpoint

The sole ClickHouse `RowBinaryWithNamesAndTypes` decoder now projects `Date`,
`Date32`, `DateTime`, and `DateTime64` into the shared bounded `Temporal` value
kind. These values no longer masquerade as signed or unsigned integers.

## Decisions

`Date` and `Date32` are calendar days from the Unix epoch and use the shared
proleptic-Gregorian formatter also used by PostgreSQL. `DateTime` and
`DateTime64` are epoch instants and canonicalize to UTC with `Z`. Exact timezone
declarations remain in column type metadata; presentation must not reinterpret
the epoch using its local timezone.

`DateTime64` accepts only ClickHouse's scale 0 through 9 and retains exactly
that many fractional digits, including trailing zeros. Negative ticks use
Euclidean division, preserving the instant before the Unix epoch instead of
truncating toward zero. Invalid type arguments, timezone syntax, and precision
fail closed during metadata parsing.

Recursive arrays, tuples, and maps quote canonical temporal text inside the
existing deterministic structured projection. This prevents temporal strings
from being confused with JSON numbers or syntax.

## Bounds and evidence

- Fixed-width fields are consumed without allocation before formatting.
- Canonical text copies only the caller's cell budget and records its exact
  original byte length when truncated.
- Shared formatter tests cover Unix epoch, astronomical and expanded years,
  negative fractional instants, nanoseconds, and invalid scale overflow.
- Parser tests cover timezone-bearing forms and reject missing/invalid scales,
  unquoted timezone names, and extra arguments.
- Official ClickHouse 25.8.28.1 and 26.3.17.4 LTS Testcontainers, with no
  compression and LZ4, prove Date, Date32, UTC DateTime, nanosecond DateTime64,
  bounded truncation, and recursive DateTime64 arrays.
- The same serialized matrix retains a single explicit 15-second evidence
  budget for synchronous server-confirmed cancellation; this replaces a
  scheduling-sensitive five-second fixture literal without changing product
  cancellation semantics.

Temporal input/edit round trips, presentation, and UniFFI remain later work.

External concepts: ClickHouse RowBinary temporal encodings, Unix epochs, DateTime64 scale/timezone metadata
Public sources: <https://clickhouse.com/docs/sql-reference/data-types/date>, <https://clickhouse.com/docs/sql-reference/data-types/date32>, <https://clickhouse.com/docs/sql-reference/data-types/datetime>, <https://clickhouse.com/docs/sql-reference/data-types/datetime64>
Implementation source: ClickHouse primary documentation, official clickhouse-rs 0.15.1 source/tests, and TableRock-owned decoder/tests
Copied code/assets/text: none
