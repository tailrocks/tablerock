# Phase 2 PostgreSQL Numeric Decoder Evidence

## Checkpoint

The sole PostgreSQL binary `RowStream` path now decodes `numeric` into the core
`Decimal` kind on PostgreSQL 17.10 and 18.4. The projection preserves arbitrary
precision, declared scale, trailing fractional zeros, sign, scaled zero, `NaN`,
`Infinity`, and `-Infinity` without a floating-point conversion.

## Wire decision

TableRock implements PostgreSQL's primary binary contract directly: unsigned
digit count, signed base-10000 weight, sign, declared scale, then big-endian
base-10000 digits. Accepted finite signs are positive and negative; accepted
special signs are NaN and both infinities. Scale bits and every digit are
validated before projection.

Special values follow PostgreSQL receive semantics: only the special sign
determines the value after the header itself passes validation. Finite values
render the declared decimal scale exactly. Digits hidden by that scale are
truncated before sign selection, so hostile negative-zero input normalizes to
positive zero like PostgreSQL.

## Bounds and failure

- projection writes only while the complete decimal fits the cell limit;
- a valid value whose canonical decimal exceeds that limit remains bounded
  `Unknown` with exact PostgreSQL type and raw truncation truth;
- malformed header length, sign, scale, or base-10000 digit becomes bounded
  `Invalid` with exact type identity;
- no numeric dependency, machine integer, or floating-point approximation is
  introduced;
- the driver still receives one complete field before decoder bounds apply.

## Evidence

Unit tests cover positive and negative scales, trailing zeros, scaled zero,
hidden negative digits, all special values, insufficient cell capacity, and
malformed headers/digits. Official `postgres:17.10-alpine` and
`postgres:18.4-alpine` Testcontainers fixtures prove `123.450`, `-0.0012300`,
`12345678901234567890.1234567890`, all special values, and `0.000` through the
same immutable page path.

Context7 was attempted first and reported its monthly quota exhausted. Wire and
special-value behavior was verified against PostgreSQL 18 `numeric_send` and
`numeric_recv` primary source, PostgreSQL numeric documentation, and pinned
tokio-postgres 0.7.18 source.

External concepts: PostgreSQL numeric binary format, base-10000 arbitrary precision, declared decimal scale
Public sources: <https://github.com/postgres/postgres/blob/REL_18_STABLE/src/backend/utils/adt/numeric.c>, <https://www.postgresql.org/docs/current/datatype-numeric.html>, <https://docs.rs/tokio-postgres/0.7.18>
Implementation source: TableRock-owned bounded adapter decoder and independent Testcontainers fixtures
Copied code/assets/text: none
