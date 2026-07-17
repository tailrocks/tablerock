# Phase 2 PostgreSQL Temporal Decoder Evidence

## Checkpoint

The sole PostgreSQL binary `RowStream` path decodes `date`, `time`, `timestamp`,
and `timestamptz` into the shared bounded `Temporal` value kind. Exact
PostgreSQL type identity stays in column metadata; no driver type crosses the
adapter boundary.

## Decision

Decoding is dependency-free because PostgreSQL's date range exceeds common
host date-time library ranges. The adapter converts the PostgreSQL 2000-01-01
epoch with proleptic-Gregorian integer arithmetic and emits:

- ISO calendar dates, with astronomical signed years available for the full
  binary domain;
- `HH:MM:SS` with exact six-digit fractions when microseconds are nonzero;
- `T`-separated timestamps;
- UTC-normalized timestamptz values with a `Z` suffix; and
- literal `infinity` and `-infinity` sentinels.

PostgreSQL stores timestamptz instants in UTC, so the projection is independent
of session `TimeZone`. Unzoned timestamp remains unzoned. `time` accepts the
documented exact `24:00:00` boundary but rejects values outside the day.

## Bounds, failure, and evidence

Canonical text is produced from fixed-width wire fields, then copied only up to
the caller's cell budget with exact original length. Wrong widths and invalid
time-of-day ranges become bounded `Invalid` values; tight output budgets remain
honest truncated `Temporal` values. Errors and Debug output contain no cell
content.

Unit tests prove both epochs, negative-microsecond day rollover, UTC suffix,
24:00, infinities, truncation, malformed widths, and invalid time ranges.
Official `postgres:17.10-alpine` and `postgres:18.4-alpine` Testcontainers prove
leap date, precise time, local timestamp, offset-to-UTC timestamptz conversion,
and positive/negative infinity through immutable result pages.

Research 177 subsequently adds `timetz`, interval, and live BC/expanded-year
coverage without flattening their native component semantics.

External concepts: PostgreSQL temporal storage domains, UTC timestamptz semantics, and special values
Public sources: <https://www.postgresql.org/docs/current/datatype-datetime.html>, <https://docs.rs/tokio-postgres/0.7.18>
Implementation source: PostgreSQL primary documentation, pinned tokio-postgres source, and TableRock-owned decoder/tests
Copied code/assets/text: none
