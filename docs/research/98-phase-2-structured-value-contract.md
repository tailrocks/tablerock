# Phase 2 Structured Value Contract

## Decision

The shared Rust value vocabulary now has a dedicated `Structured` kind for
database-native containers. Arrays, tuples, maps, nested records, JSON trees,
and future structured engine values must not masquerade as ordinary text and
must not be downgraded to `Unknown` after their type is understood.

`OwnedValue::structured` carries a bounded UTF-8 canonical projection plus the
same complete/truncated fact used by text and binary values. The immutable page
encoding assigns it a distinct value-kind tag while retaining the existing
offset, arena, and truncation representation. This keeps pages coarse and
UniFFI-ready without a per-node or per-cell object graph.

## Invariants

- Structured projections are UTF-8 and validated when pages are assembled or
  decoded.
- Truncation is legal and records an optional original projected byte length.
- Debug output reveals only kind and truncation, never container content.
- Ordinary text remains semantically distinct; presentation does not infer a
  container by parsing arbitrary cell text.
- Binary, invalid, and unknown values retain their existing meanings.
- No database-client type enters the core contract.

## Evidence

Core value tests prove construction, semantic kind, truncation, and redacted
debug output. Page encoding tests prove accepted UTF-8 structured payloads and
the existing hostile offset/length validation around the new kind. The next
ClickHouse checkpoint projects recursive RowBinary containers through this
contract.

External concepts: typed structured values, bounded canonical projection
Public sources: <https://clickhouse.com/docs/sql-reference/data-types>, <https://clickhouse.com/docs/interfaces/formats/RowBinary>
Implementation source: TableRock-owned core contract and tests
Copied code/assets/text: none
