# Phase 2 PostgreSQL Domain Projection Evidence

## Checkpoint

The PostgreSQL adapter now decodes domain values through their underlying
semantic contracts. Scalar, nested-domain, array, range, and composite domains
are covered without exposing database-client types beyond the adapter.

## Decision

A successfully decoded domain produces the same core value kind as its
underlying type. Domain identity belongs to existing column or composite-field
metadata, not a second wrapper around every value. If underlying decoding is
invalid or unknown, the raw fallback is rebuilt with the outer domain's engine
type so diagnostic identity is not lost. Every domain layer consumes the shared
64-level structured nesting budget.

PostgreSQL RowDescription reports base type OIDs for top-level domain
expressions and stored domain columns. TableRock cannot recover that domain
identity from the result protocol and does not infer it. Named composite field
metadata does preserve domain OIDs/types, providing the real-server evidence
path for exact domain identity.

## Bounds and failure truth

- underlying value bounds and failure classifications remain authoritative;
- invalid underlying bytes become `Invalid` with outer domain identity;
- unsupported or over-budget underlying projection becomes `Unknown` with
  outer domain identity;
- nested domains consume the shared 64-level recursion ceiling;
- successful scalar/structured values remain bounded by their existing
  contracts and caller cell/page limits.

## Evidence

Unit tests cover integer, nested-domain, and array-domain success; malformed
integer bytes, unsupported underlying values, cell bounds, and nesting depth;
and exact outer failure identity. Testcontainers Rust 0.27.3 owns official
`postgres:17.10-alpine` and `postgres:18.4-alpine` fixtures. Both lines prove
domain-bearing named composite fields over scalar, nested domain, array, range,
and composite underlying types, with exact user-defined OID/name metadata and
canonical semantic values. Independent attempts using explicit casts and
stored table columns prove top-level RowDescription flattening to base types.

## Remaining work

Enum and additional scalar-family projection remain separate typed-value
checkpoints. Domain editors, public parameter plans and request bounds,
service/UI projection, and UniFFI remain open. The PostgreSQL driver still
receives a complete field before TableRock applies its cell bound; strict
pre-driver field allocation remains open.

Context7 was attempted first and reported its monthly quota exhausted. Current
API behavior was therefore checked against PostgreSQL primary domain/type-system
documentation and docs.rs/pinned `postgres-types` 0.2.14 `Kind::Domain` source.

External concepts: PostgreSQL domain underlying types, constraints, RowDescription flattening, composite field identity
Public sources: <https://www.postgresql.org/docs/current/domains.html>, <https://www.postgresql.org/docs/current/extend-type-system.html>, <https://www.postgresql.org/docs/current/protocol-message-formats.html>, <https://docs.rs/postgres-types/0.2.14/postgres_types/enum.Kind.html>
Implementation source: TableRock-owned adapter and independent Testcontainers fixtures
Copied code/assets/text: none
