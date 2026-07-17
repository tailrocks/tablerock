# Phase 2 PostgreSQL Bit-String Projection Evidence

## Checkpoint

The PostgreSQL adapter now projects fixed `bit` and variable `varbit` binary
values into bounded canonical text without exposing client types through core
contracts.

## Decision

One logical bit becomes one ASCII `0` or `1` byte in the shared `Text` contract.
The decoder validates the signed big-endian bit count, exact ceiling-divided
payload length, and zero unused low bits in the final wire byte. It produces
only the caller-bounded prefix instead of first expanding the complete bit
string. Truncation records the full logical bit count as canonical byte length.

Type modifiers describe declared fixed/maximum lengths but are not required to
decode a server-produced value. TableRock retains `bit` versus `varbit` in
column metadata and does not invent missing modifier facts inside each cell.

## Bounds and failure truth

- canonical allocation never exceeds the caller cell limit;
- empty `varbit` is complete empty `Text`;
- negative counts, short headers, count/payload mismatch, trailing bytes, and
  nonzero padding bits become `Invalid` with exact PostgreSQL type identity;
- raw bytes and canonical bit text are never logged.

## Evidence

Unit tests cover fixed, variable, empty, multi-byte, truncated, negative-count,
short-header, missing/extra-byte, and nonzero-padding cases. Testcontainers Rust
0.27.3 owns official `postgres:17.10-alpine` and `postgres:18.4-alpine`
fixtures. Both lines prove `bit(8)`, short/empty/multi-byte `varbit`, exact type
identity, and complete canonical `Text` values.

## Remaining work

Research 187 subsequently closes unsigned identifier projection. Additional
scalar families remain separate typed-value work. Bit-string editors and type-modifier schema metadata, public parameter plans and request bounds,
service/UI projection, and UniFFI remain open. The PostgreSQL driver still
receives a complete field before TableRock applies its cell bound; strict
pre-driver field allocation remains open.

Context7 was attempted first and reported its monthly quota exhausted. Current
behavior was therefore checked against PostgreSQL 18 primary bit-string docs
and pinned `postgres-protocol` 0.6.12 `varbit_from_sql` plus `postgres-types`
0.2.14 bit-vector integration source.

External concepts: PostgreSQL bit-string binary length, MSB-first payload, padding
Public sources: <https://www.postgresql.org/docs/current/datatype-bit.html>, <https://docs.rs/postgres-protocol/0.6.12/postgres_protocol/types/fn.varbit_from_sql.html>, <https://docs.rs/postgres-types/0.2.14/postgres_types/trait.FromSql.html>
Implementation source: TableRock-owned adapter and independent Testcontainers fixtures
Copied code/assets/text: none
