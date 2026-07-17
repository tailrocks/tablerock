# Phase 2 PostgreSQL Multirange Projection Evidence

## Checkpoint

The sole PostgreSQL binary `RowStream` path now projects generic multiranges as
bounded canonical `Structured` values on PostgreSQL 17.10 and 18.4. The adapter
uses `Kind::Multirange` subtype metadata and the existing range/scalar decoders
without exposing database-client types.

## Decision

A multirange is one tagged ordered collection of canonical ranges:

```json
{"$multirange":[{"$range":{"empty":false,"lower":{"kind":"inclusive","value":1},"upper":{"kind":"exclusive","value":3}}}]}
```

The empty multirange is `{"$multirange":[]}`. Member order is PostgreSQL's
canonical server order. Each member retains the explicit empty, unbounded,
inclusive, and exclusive contract from research 180; subtype values reuse the
same scalar and nested-structured projections.

## Bounds and failure truth

- at most 1,000,000 declared range members are accepted;
- each member has an unsigned 32-bit length and must contain exactly one valid
  range payload;
- the existing cell limit bounds the complete canonical JSON projection and
  records its full original length when truncated;
- truncated counts/lengths/members, invalid member ranges, and trailing bytes
  classify the whole multirange as `Invalid`;
- an over-budget count or unsupported member projection remains whole-value
  `Unknown`, never a partial member list.

## Evidence

Unit tests cover empty and ordered multiranges, bounded output, missing count,
missing/truncated lengths and payloads, invalid member ranges, trailing bytes,
and the member ceiling. Testcontainers Rust 0.27.3 owns official
`postgres:17.10-alpine` and `postgres:18.4-alpine` fixtures. Both lines prove
empty and nonempty `int4multirange`, unbounded `int8multirange`, exact decimal
`nummultirange`, and temporal `datemultirange` through the same typed stream.

## Remaining work

Research 182 subsequently closes composite projection. Domain projection
remains adapter-private unknown. Multirange editors, public parameter plans and
request bounds, service/UI
projection, and UniFFI remain open. The PostgreSQL driver still receives a
complete field before TableRock applies its cell bound; strict pre-driver field
allocation remains open.

Context7 was attempted first and reported its monthly quota exhausted. Current
API behavior was therefore checked against PostgreSQL primary multirange docs
and source plus docs.rs/pinned `postgres-types` 0.2.14 source.

External concepts: PostgreSQL multirange binary count/length framing and canonical member order
Public sources: <https://www.postgresql.org/docs/current/rangetypes.html>, <https://www.postgresql.org/docs/current/functions-range.html>, <https://doxygen.postgresql.org/multirangetypes_8c_source.html>, <https://docs.rs/postgres-types/0.2.14/postgres_types/enum.Kind.html>
Implementation source: TableRock-owned adapter and independent Testcontainers fixtures
Copied code/assets/text: none
