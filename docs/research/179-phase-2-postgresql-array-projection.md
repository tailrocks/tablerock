# Phase 2 PostgreSQL Array Projection Evidence

## Checkpoint

The sole PostgreSQL binary `RowStream` path now projects generic arrays as
bounded canonical `Structured` values on PostgreSQL 17.10 and 18.4. The adapter
retains every declared dimension and lower bound, reconstructs row-major nested
values, preserves NULL elements, and recursively uses the existing scalar
decoders without exposing database-client types.

## Decision

An array has one tagged canonical representation:

```json
{"$array":{"dimensions":[[0,3]],"values":[7,8,9]}}
```

Each dimension pair is `[lower_bound, length]`. This retains PostgreSQL facts
that a plain JSON array erases. Values nest in row-major order. Zero-dimensional
empty arrays use empty `dimensions` and `values` lists. Scalar values use their
existing core projection; binary, exact-float, decimal, and other structured
kinds retain explicit tags where ordinary JSON would lose type or precision.

## Bounds and failure truth

- at most 64 dimensions and 1,000,000 declared elements are accepted;
- the existing cell limit bounds the complete canonical JSON projection and
  records its full original length when truncated;
- invalid dimension counts or lengths, invalid null flags, element OID
  mismatches, illegal element lengths, contradictory NULL flags, truncated
  payloads, and trailing bytes classify the whole array as `Invalid`;
- a structurally valid array containing an unsupported element kind, or one
  exceeding the structural budget, remains whole-value `Unknown` rather than
  presenting a partial projection as complete.

## Evidence

Unit tests cover zero-dimensional empty arrays, nullable vectors, non-default
and negative lower bounds, multidimensional nesting, bounded output, every
structural rejection above, and the element budget. Testcontainers Rust 0.27.3
owns official `postgres:17.10-alpine` and `postgres:18.4-alpine` fixtures. Both
lines prove nullable integer arrays, a 2x2 matrix, lower bound zero, escaped and
Unicode text, temporal elements, and a prepared `int4[]` result through the
same typed stream.

## Remaining work

Composite, range, and domain element projection remain adapter-private unknown
values. Array editors, public parameter plans and aggregate request bounds,
service/UI projection, and UniFFI remain open. The PostgreSQL driver still
receives a complete field before TableRock applies its cell bound; strict
pre-driver field allocation remains open.

Context7 was attempted first and reported its monthly quota exhausted. Current
API behavior was therefore checked against PostgreSQL primary array and binary
protocol documentation, docs.rs `postgres-types` 0.2.14 `Kind`, and pinned
`tokio-postgres`/`postgres-types` source.

External concepts: PostgreSQL array binary representation, dimensions, lower bounds, row-major order
Public sources: <https://www.postgresql.org/docs/current/arrays.html>, <https://docs.rs/postgres-types/0.2.14/postgres_types/enum.Kind.html>, <https://www.postgresql.org/docs/current/protocol-message-formats.html>
Implementation source: TableRock-owned adapter and independent Testcontainers fixtures
Copied code/assets/text: none
