# Phase 2 PostgreSQL NULL and Array Parameter Evidence

Date: 2026-07-17

## Decision

NULL parameters retain a declared PostgreSQL type and produce a core null cell.
Structured PostgreSQL parameters are encoded by the official client; until a
lossless structured decoder owns that type, the result remains bounded
`Unknown` with exact engine type identity and raw binary payload. TableRock does
not stringify arrays or infer a lossy cross-engine structure.

## Evidence

The PostgreSQL 17.10/18.4 parameter matrix now binds two additional values in the
same prepared statement:

- `Option<&str>::None` cast to `text` produces a null cell;
- `Vec<i32>` cast to `int4[]` round-trips as bounded `Unknown` with engine type
  `_int4` and a non-empty binary payload.

The page remains one final row under six-column, arena, and cell bounds. This
proves official-client encoding, declared NULL typing, array transport, and
honest fallback on both supported lines.

This closes NULL and integer-array parameter feasibility. Public reviewed
parameter plans, parameter count/aggregate-byte bounds, composite/JSON/range
parameter breadth, structured result decoding, statement lifecycle/cache,
notices, multiple statements, COPY, reconnect, ambiguous writes, presentation,
and UniFFI remain open.

## Safety contract

- NULL never becomes empty text or a sentinel string.
- Structured values never receive invented lossy normalization.
- Raw fallback bytes remain bounded and carry exact engine type metadata.
- Parameter values remain absent from errors, logs, history, and diagnostics.
- Failed parameter operations are not automatically replayed.

## Provenance

External concepts: PostgreSQL typed NULL and array parameter encoding
Public sources: <https://docs.rs/postgres-types/latest/postgres_types/trait.ToSql.html>
and <https://www.postgresql.org/docs/current/arrays.html>
TableRock requirements: research 01, 06, 10, 14, 20, 30, 31, 32, 87, 98, 157
Implementation source: TableRock-owned fixed probes and bounded unknown-value
contract
Copied code/assets/text: none
