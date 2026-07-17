# Phase 2 PostgreSQL LSN Projection Evidence

## Checkpoint

The PostgreSQL adapter now projects `pg_lsn` binary values into bounded
canonical WAL-location text.

## Decision

An LSN is an unsigned 64-bit byte position, but its domain meaning is a WAL
location. TableRock therefore emits PostgreSQL's canonical uppercase
`HIGH/LOW` hexadecimal form as shared `Text`, rather than exposing a generic
integer. Each half uses up to eight hexadecimal digits without leading zeros.

## Bounds and failure truth

- the wire payload must contain exactly eight big-endian bytes;
- short or trailing payloads become `Invalid` with `pg_lsn` identity;
- canonical output is bounded by the caller cell limit and truncation records
  its complete original byte length;
- LSN values are never logged.

## Evidence

Unit tests cover zero, representative, maximum, bounded, short, and trailing
payloads. Testcontainers Rust 0.27.3 owns official `postgres:17.10-alpine` and
`postgres:18.4-alpine` fixtures. Both lines prove `0/0`, `16/B374D848`, and
`FFFFFFFF/FFFFFFFF` with exact type identity and complete canonical `Text`.

## Remaining work

Research 189 subsequently closes tuple-identifier projection. OID vectors,
snapshots, and additional scalar families remain separate typed-value work. LSN editors, public parameter plans and
request bounds, service/UI projection, and UniFFI remain open.

Context7 was attempted earlier in this checkpoint sequence and reported its
monthly quota exhausted. Current behavior was therefore checked against
PostgreSQL 18 primary `pg_lsn` documentation plus pinned `postgres-types`
0.2.14 `PgLsn` and `postgres-protocol` 0.6.12 LSN source.

External concepts: PostgreSQL WAL log sequence number wire and canonical output
Public sources: <https://www.postgresql.org/docs/current/datatype-pg-lsn.html>, <https://docs.rs/postgres-types/0.2.14/postgres_types/struct.PgLsn.html>, <https://docs.rs/postgres-protocol/0.6.12/postgres_protocol/types/fn.lsn_from_sql.html>
Implementation source: TableRock-owned adapter and independent Testcontainers fixtures
Copied code/assets/text: none
