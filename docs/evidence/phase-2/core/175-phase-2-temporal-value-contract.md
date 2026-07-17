# Phase 2 Temporal Value Contract

## Checkpoint

The shared Rust value and immutable-page contracts now distinguish temporal
values from ordinary text. `OwnedValue::temporal` carries a bounded canonical
UTF-8 projection plus exact truncation state, and page encoding transports the
same `Temporal` kind without database-client types.

## Decision

Temporal semantics are a stable cross-engine concern. Presentation must not
infer them from PostgreSQL or ClickHouse type-name strings. Exact native type
identity remains column metadata, while the value kind selects common display,
copy, filtering, and future editor behavior.

The payload remains canonical text rather than a fixed epoch integer because
the supported engines expose different temporal domains: zoned and unzoned
instants, dates, times, intervals, precision, infinity, and engine-specific
ranges. One integer representation would erase required meaning. Canonical
forms are defined by each decoder checkpoint and remain bounded before crossing
adapter or UniFFI boundaries.

## Bounds and evidence

- Construction rejects impossible truncation metadata.
- Page validation requires UTF-8 and permits truncation only with valid original
  length truth.
- Columnar page bytes remain immutable and allocation-bounded.
- Structured projection quotes temporal text, preventing container ambiguity.
- Debug output reports only kind and truncation, never values.

Core and engine unit suites prove construction, every-kind page transport,
malformed UTF-8 rejection, and exhaustive consumer handling. PostgreSQL and
ClickHouse decoder promotion remains explicitly open.

External concepts: PostgreSQL temporal domains and internal interval components
Public sources: <https://www.postgresql.org/docs/current/datatype-datetime.html>
Implementation source: TableRock-owned shared value/page contracts and tests
Copied code/assets/text: none
