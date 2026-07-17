# Phase 2 PostgreSQL Typed Stream Evidence

## Checkpoint

The PostgreSQL adapter now has one result path: extended-query `RowStream`
frames decoded from binary protocol values into bounded, immutable core pages.
The earlier simple-query text stream has been removed. Driver `Row`, `Column`,
`Type`, `Statement`, and `RowStream` types remain private to the adapter.

This is not the complete PostgreSQL spike. Decimal and temporal decoding,
structured complex-value decoding, authentication taxonomy, late errors, and
reconnect ownership remain required. Verified TLS
and client identity later pass in
[`136-phase-2-postgresql-tls-identity.md`](136-phase-2-postgresql-tls-identity.md).
Typed scalar parameters subsequently pass in research 157.
Bounded notices subsequently pass in research 159.

## Type decision

The adapter requests raw binary payloads through an internal `FromSql` decoder
that accepts every PostgreSQL type. It then makes one lossless classification:

| PostgreSQL value | Core value |
|---|---|
| `bool` | Boolean |
| `int2`, `int4`, `int8` | signed 64-bit integer |
| `float4`, `float8` | exact Float64 bits; Float32 is widened exactly |
| `text`, `varchar`, `bpchar`, `name` | UTF-8 text |
| `bytea` | binary |
| NULL of any type | null |
| `numeric` | exact arbitrary-precision decimal text; research 172 |
| `uuid` | canonical lowercase hyphenated text; research 174 |
| generic array of supported values | canonical structured dimensions, lower bounds, and nested row-major values; research 179 |
| generic range of supported values | canonical structured empty state and explicit bound kinds/values; research 180 |
| generic multirange of supported values | ordered canonical structured range members; research 181 |
| named composite or anonymous record | canonical ordered fields with name/null-name, OID, type, and nested value; research 182 |
| domain with supported underlying value | underlying semantic value; domain identity in column/field metadata where PostgreSQL supplies it; research 183 |
| user-defined enum | catalog-validated bounded UTF-8 text with exact column type identity; research 184 |
| `inet`, `cidr`, `macaddr`, `macaddr8` | strictly validated bounded canonical network text; research 185 |
| fixed `bit`, varying `varbit` | length/padding-validated bounded canonical bit text; research 186 |
| valid unsupported type | unknown with PostgreSQL type name and raw binary payload |
| malformed payload for a known type | invalid with PostgreSQL type name and raw payload |

Unsupported does not mean discarded. Unknown anonymous field OIDs retain the
whole record's binary representation as an `UnknownValue`;
later type-specific decoders can replace that classification without changing
the page or adapter boundary. Research 167 adds the raw complex-value and
large-binary matrix; research 168 subsequently promotes JSON/JSONB to bounded
canonical `Structured` projections, research 179 promotes generic arrays while
preserving their PostgreSQL dimensions and lower bounds, and research 180
promotes generic ranges with explicit bound truth.
Research 181 composes the same range truth through generic multiranges.
Research 182 promotes named composites and anonymous records while preserving
self-describing field identity.
Research 183 decodes domains recursively and records PostgreSQL's top-level
RowDescription domain-flattening limit.
Research 184 projects user-defined enum labels as bounded text and rejects
payloads absent from pinned catalog metadata.
Research 185 projects PostgreSQL network address families as canonical bounded
text after strict binary-envelope and CIDR-network validation.
Research 186 projects fixed/varying bit strings without expanding beyond the
caller bound and rejects invalid counts, framing, or unused padding.
Malformed known values are never silently treated as valid.

Column metadata carries the server type name and conservatively marks columns
nullable because PostgreSQL row descriptions do not carry nullability facts.

## Bounds and failure

- The adapter prepares only fixed read-only probes. No raw SQL bypass exists.
- Page row, column, arena, column-text, and per-cell byte limits are enforced.
- Text truncation remains UTF-8 scalar-safe. Binary, unknown, and invalid
  payloads retain exact original byte length when truncated.
- A fixed-width decoded value needs its canonical core width. If the remaining
  page arena cannot hold it, the adapter emits an empty, truncated unknown value
  instead of exceeding the page bound or inventing a partial integer.
- The driver necessarily receives one complete PostgreSQL data row before the
  adapter can inspect `raw_size_bytes`; this checkpoint makes no constant-memory
  claim for an individually unbounded server field.
- Driver errors remain message-free public categories. Page warnings explicitly
  report row, byte, unknown, and invalid conditions.

## Testcontainers support matrix

| Server | Real fixture evidence | Claim |
|---|---|---|
| PostgreSQL 17.10 | official `postgres:17.10-alpine`; extended-query preparation and streaming; Boolean, signed integers, Float32/Float64, exact numeric, canonical UUID, complete scalar temporal family, text, binary, NULL, JSON/JSONB structured projection, array/range/record unknown preservation, truncation | typed tracer |
| PostgreSQL 18.4 | same typed suite on official `postgres:18.4-alpine`; existing bounded paging and cancellation suites also pass | typed tracer |

Testcontainers Rust 0.27.3 owns both fixture lifecycles and ephemeral mapped
ports. Trust authentication and disabled TLS in this historical typed suite
remain disposable-fixture facts; the separate supported-line TLS suite now
proves custom roots, server-name verification, mTLS, and downgrade rejection.

## Verification record

- Known/malformed/insufficient-capacity decoder unit tests: pass.
- PostgreSQL 17.10 and 18.4 typed Testcontainers suite: pass.
- PostgreSQL 18.4 bounded paging and cancellation Testcontainers suites: pass.
- Full workspace, lint, documentation, dependency, secret, English, and drift
  gates are recorded in the publishing commit.

Context7 was attempted first and reported its monthly quota exhausted. API
behavior was verified against pinned `tokio-postgres` 0.7.18 source and docs,
plus PostgreSQL primary protocol and type documentation.

External concepts: PostgreSQL extended query flow, binary data-row formats, type OIDs, SQL NULL
Public sources: <https://docs.rs/tokio-postgres/0.7.18>, <https://www.postgresql.org/docs/current/protocol-message-formats.html>, <https://www.postgresql.org/docs/current/datatype.html>
Implementation source: TableRock-owned adapter, core page contracts, and independent Testcontainers fixtures
Copied code/assets/text: none
