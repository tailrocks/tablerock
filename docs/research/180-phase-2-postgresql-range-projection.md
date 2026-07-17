# Phase 2 PostgreSQL Range Projection Evidence

## Checkpoint

The sole PostgreSQL binary `RowStream` path now projects generic ranges as
bounded canonical `Structured` values on PostgreSQL 17.10 and 18.4. The adapter
uses `Kind::Range` subtype metadata and the existing scalar decoders without
exposing database-client types.

## Decision

Empty and nonempty ranges use one tagged JSON family:

```json
{"$range":{"empty":true}}
{"$range":{"empty":false,"lower":{"kind":"inclusive","value":1},"upper":{"kind":"unbounded"}}}
```

Every finite bound states `inclusive` or `exclusive`; an absent bound states
`unbounded`. This avoids conflating unbounded with SQL NULL and preserves facts
that PostgreSQL range text formatting or a two-element array would erase.
Subtype values reuse the same explicit exact-float, decimal, binary, temporal,
text, nested-structured, and integer projections as arrays.

## Bounds and failure truth

- the existing cell limit bounds the complete canonical JSON projection and
  records its full original length when truncated;
- each decoded finite bound has an 8 MiB internal projection ceiling, allowing
  the outer writer to report honest truncation without unbounded expansion;
- unknown flag bits, mixed empty flags, inclusive-unbounded contradictions,
  negative or truncated bound lengths, missing bounds, and trailing bytes
  classify the whole range as `Invalid`;
- a structurally valid range whose subtype cannot be projected completely
  remains whole-value `Unknown`, never a partially understood range.

## Evidence

Unit tests cover empty, finite inclusive/exclusive, unbounded, scalar values,
bounded output, and every structural rejection above. Testcontainers Rust
0.27.3 owns official `postgres:17.10-alpine` and `postgres:18.4-alpine`
fixtures. Both lines prove canonical server behavior for `int4range`,
`int8range`, `numrange`, `daterange`, bounded and empty `tstzrange`, plus an
array of `int4range`, through the same typed stream. Discrete-range
canonicalization is retained from server output, including
`(,42]::int8range` becoming an exclusive upper bound at 43; timestamp bounds
reuse UTC normalization.

## Remaining work

Research 181 subsequently composes ranges through generic multiranges.
Composite and domain projection remain adapter-private unknown values. Range
editors, public parameter plans and request bounds, service/UI
projection, and UniFFI remain open. The PostgreSQL driver still receives a
complete field before TableRock applies its cell bound; strict pre-driver field
allocation remains open.

Context7 was attempted first and reported its monthly quota exhausted. Current
API behavior was therefore checked against PostgreSQL primary range docs and
source, docs.rs `postgres-types` 0.2.14 `Kind`, and pinned
`postgres-protocol` 0.6.12 source.

External concepts: PostgreSQL binary range flags, bound kinds, discrete canonicalization
Public sources: <https://www.postgresql.org/docs/current/rangetypes.html>, <https://www.postgresql.org/docs/current/functions-range.html>, <https://doxygen.postgresql.org/rangetypes_8h_source.html>, <https://docs.rs/postgres-types/0.2.14/postgres_types/enum.Kind.html>
Implementation source: TableRock-owned adapter and independent Testcontainers fixtures
Copied code/assets/text: none
