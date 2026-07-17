# Phase 2 PostgreSQL Composite Projection Evidence

## Checkpoint

The sole PostgreSQL binary `RowStream` path now projects named composites and
anonymous `record` values as bounded canonical `Structured` fields on
PostgreSQL 17.10 and 18.4. Named types use `Kind::Composite` field metadata;
anonymous records resolve their self-describing wire OIDs without exposing
database-client types beyond the adapter.

## Decision

Both forms use one tagged ordered field collection:

```json
{"$composite":{"fields":[{"name":"id","oid":23,"type":"int4","value":7}]}}
```

Anonymous fields use `"name":null`. Every field retains wire OID and canonical
type name, avoiding positional-only ambiguity. NULL is a field value, not a
missing field. Nested arrays, ranges, multiranges, and composites reuse their
existing tagged projections.

## Bounds and failure truth

- at most 1,664 fields are accepted;
- all recursive PostgreSQL structured values share a 64-level nesting cap;
- finite field projection has the existing 8 MiB internal component ceiling;
- the caller's cell limit bounds complete canonical JSON and records its full
  original length when truncated;
- named field count/OID mismatches, invalid negative lengths, truncated fields,
  malformed known field values, and trailing bytes make the whole value
  `Invalid`;
- unknown anonymous OIDs, over-budget fields/depth, or unsupported nested
  projections preserve the whole composite as `Unknown`.

## Evidence

Unit tests cover named and anonymous fields, escaped/Unicode names and values,
NULL, bounded output, count/OID/length/value/trailing failures, unknown OIDs,
field count, and depth limits. Testcontainers Rust 0.27.3 owns official
`postgres:17.10-alpine` and `postgres:18.4-alpine` fixtures. Both lines create
a named probe type and prove exact named metadata, anonymous self-description,
NULL, Unicode text, nested integer arrays, and nested date ranges through the
same typed stream. The earlier eight-byte record probe now produces honest
structured truncation without an unknown-value warning.

## Remaining work

Domain projection remains adapter-private unknown. Composite editors, public
parameter plans and request bounds, service/UI projection, and UniFFI remain
open. The PostgreSQL driver still receives a complete field before TableRock
applies its cell bound; strict pre-driver field allocation remains open.

Context7 was attempted first and reported its monthly quota exhausted. Current
API behavior was therefore checked against PostgreSQL primary `record_send`/
`record_recv` source and docs.rs/pinned `postgres-types` 0.2.14 `Kind`/`Field`
source.

External concepts: PostgreSQL composite binary field count/OID/length framing and anonymous record self-description
Public sources: <https://doxygen.postgresql.org/rowtypes_8c_source.html>, <https://www.postgresql.org/docs/current/rowtypes.html>, <https://docs.rs/postgres-types/0.2.14/postgres_types/enum.Kind.html>, <https://docs.rs/postgres-types/0.2.14/postgres_types/struct.Field.html>
Implementation source: TableRock-owned adapter and independent Testcontainers fixtures
Copied code/assets/text: none
