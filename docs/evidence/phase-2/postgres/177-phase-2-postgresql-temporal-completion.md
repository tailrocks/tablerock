# Phase 2 PostgreSQL Temporal Completion Evidence

## Checkpoint

The sole PostgreSQL binary result path now covers every scalar temporal family:
`date`, `time`, `timetz`, `timestamp`, `timestamptz`, and `interval`. Values
cross the adapter only as bounded shared `Temporal`; exact PostgreSQL type names
remain column metadata.

## Decisions

`timetz` retains its local clock and explicit numeric offset. PostgreSQL sends
offset seconds west of UTC, while canonical text uses the conventional
east-positive sign. TableRock accepts the documented 15:59:59 absolute offset
domain and writes seconds only when nonzero.

Intervals remain three independent native components because months cannot be
truthfully converted to days and calendar days cannot always be converted to
24-hour durations. Canonical text is
`P{months}M{days}DT{signed-seconds[.microseconds]}S`; each component retains its
own sign. PostgreSQL interval infinity is recognized only by the structural
all-maximum or all-minimum three-field sentinel. Other extreme microsecond
values remain finite exact intervals.

Dates use a proleptic Gregorian astronomical year: PostgreSQL `0001 BC` becomes
year `0000`, and years beyond four digits carry an explicit positive sign. This
gives one locale- and `DateStyle`-independent representation.

## Bounds, failure, and evidence

- `timetz` is exactly 12 bytes and interval exactly 16; wrong widths are
  bounded `Invalid`.
- Time-of-day and offset ranges fail closed.
- Canonical output obeys the caller's cell budget and records original length.
- Unit tests cover offset direction, mixed interval signs, microseconds,
  structural infinities, malformed widths/ranges, BC, and expanded-year math.
- Official PostgreSQL 17.10 and 18.4 Testcontainers prove `+06:30` timetz,
  mixed-sign interval, both interval infinities, `0001 BC`, and year 10000
  through immutable result pages.

Research 179/180 subsequently compose temporal values through generic arrays
and ranges. Typed edit/parameter round trips, presentation, and UniFFI remain
later checkpoints.

External concepts: PostgreSQL timetz offset direction, interval binary components/sentinels, proleptic Gregorian calendar
Public sources: <https://www.postgresql.org/docs/current/datatype-datetime.html>, <https://github.com/postgres/postgres>, <https://docs.rs/tokio-postgres/0.7.18>
Implementation source: PostgreSQL primary documentation/source, pinned protocol source, and TableRock-owned decoder/tests
Copied code/assets/text: none
