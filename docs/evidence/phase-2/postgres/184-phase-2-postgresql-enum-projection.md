# Phase 2 PostgreSQL Enum Projection Evidence

## Checkpoint

The PostgreSQL adapter now projects user-defined enum values as bounded text
while retaining exact enum type identity in column metadata.

## Decision

PostgreSQL sends an enum's textual label for binary results. TableRock accepts
the payload only when it is valid UTF-8 and exactly matches one label supplied
by the pinned driver's `Kind::Enum` catalog metadata. Successful values use the
shared `Text` contract. A payload absent from metadata is protocol/catalog drift,
not a new value, and becomes `Invalid` with the enum's engine type.

This avoids a redundant structured wrapper: ordering and the complete allowed
label set belong to type metadata and future schema/catalog contracts, while a
cell carries one selected label.

## Bounds and failure truth

- text output is bounded by the caller's cell limit at a UTF-8 boundary;
- truncation records the original byte length;
- invalid UTF-8 becomes `Invalid` with exact enum identity;
- labels missing from current type metadata become `Invalid` rather than being
  silently accepted;
- the adapter never logs labels or raw cell bytes.

PostgreSQL standard builds limit enum labels to 63 bytes, but TableRock still
applies caller bounds and never relies on that server build detail for memory
safety.

## Evidence

Unit tests cover Unicode success, character-safe truncation, unknown catalog
labels, invalid UTF-8, and exact failure identity. Testcontainers Rust 0.27.3
owns official `postgres:17.10-alpine` and `postgres:18.4-alpine` fixtures. Both
lines prove ASCII and Unicode labels, exact user-defined column type identity,
and complete bounded `Text` values.

## Remaining work

Research 185 subsequently closes network scalar projection. Additional scalar
families remain separate typed-value work. Enum editors and schema metadata, public parameter plans and request bounds,
service/UI projection, and UniFFI remain open. The PostgreSQL driver still
receives a complete field before TableRock applies its cell bound; strict
pre-driver field allocation remains open.

Context7 was attempted first and reported its monthly quota exhausted. Current
behavior was therefore checked against PostgreSQL 18 primary enum, `CREATE
TYPE`, and frontend/backend protocol documentation plus pinned
`postgres-types` 0.2.14 `Kind::Enum` source.

External concepts: PostgreSQL enum labels, catalog identity, binary result format
Public sources: <https://www.postgresql.org/docs/current/datatype-enum.html>, <https://www.postgresql.org/docs/current/sql-createtype.html>, <https://www.postgresql.org/docs/current/protocol-flow.html>, <https://docs.rs/postgres-types/0.2.14/postgres_types/enum.Kind.html>
Implementation source: TableRock-owned adapter and independent Testcontainers fixtures
Copied code/assets/text: none
