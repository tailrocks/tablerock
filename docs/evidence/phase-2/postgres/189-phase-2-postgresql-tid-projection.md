# Phase 2 PostgreSQL TID Projection Evidence

## Checkpoint

The PostgreSQL adapter now projects `tid` binary values into bounded structured
physical tuple locations.

## Decision

The exact canonical core form is
`{"$tid":{"block":<u32>,"offset":<u16>}}`. The wire payload is exactly one
big-endian 32-bit block number followed by one big-endian 16-bit tuple offset.
`Structured` preserves the two independently meaningful components and composes
through existing array/composite projection without a parser in presentation.

A `tid`/`ctid` identifies one physical row version, not one logical row. It can
change after update or `VACUUM FULL`. TableRock may display, inspect, filter, or
explicitly query it, but must never promote it to a durable identity, persist it
as a logical locator, or automatically use it for mutation. Mutation safety
continues to require reviewed stable keys/version predicates below presentation.

## Bounds and failure truth

- any payload other than exactly six bytes becomes `Invalid` with `tid`
  identity;
- structured output is bounded by the caller cell limit with exact canonical
  original-length truncation truth;
- tuple locations are never logged.

## Evidence

Unit tests cover first, maximum, bounded, short, and trailing payloads.
Testcontainers Rust 0.27.3 owns official `postgres:17.10-alpine` and
`postgres:18.4-alpine` fixtures. Both lines prove `(0,1)`, the maximum component
pair, and a live `pg_class.ctid` with exact type identity and structured values.

## Remaining work

Research 190 subsequently closes OID-vector projection. Snapshots and
additional scalar families remain separate typed-value work. TID filtering/inspection, stable-key mutation locators, public
parameter plans and request bounds, service/UI projection, and UniFFI remain
open.

Context7 was attempted first and reported its monthly quota exhausted. Current
behavior was therefore checked against PostgreSQL 18 primary object-identifier
and system-column docs, `REL_18_STABLE` `tidsend`, and pinned `postgres-types`
0.2.14 `Type::TID` metadata.

External concepts: PostgreSQL physical tuple identifier, ctid instability, mutation safety
Public sources: <https://www.postgresql.org/docs/current/datatype-oid.html>, <https://www.postgresql.org/docs/current/ddl-system-columns.html>, <https://github.com/postgres/postgres/blob/REL_18_STABLE/src/backend/utils/adt/tid.c>, <https://docs.rs/postgres-types/0.2.14/postgres_types/struct.Type.html>
Implementation source: TableRock-owned adapter and independent Testcontainers fixtures
Copied code/assets/text: none
