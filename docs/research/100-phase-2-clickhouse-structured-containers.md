# Phase 2 ClickHouse Structured Containers

## Decision

The one official-client `RowBinaryWithNamesAndTypes` decoder now understands
recursive `Array`, `Tuple`, `Map`, and named `Nested` signatures. Containers
cross the adapter boundary only as the core `Structured` value kind: a bounded
UTF-8 canonical projection plus complete/truncated metadata. No ClickHouse type
or per-node object graph enters the stable core contract.

Canonical projections use deterministic JSON-like scalar spelling. Arrays and
tuples are ordered arrays. Maps are ordered arrays of `[key,value]` pairs so duplicate
and non-string keys remain lossless. Binary children use an explicit lowercase
hex object (`{"$binary":"..."}`). Named tuple metadata affects the column's
engine type; values remain positional, matching RowBinary ordering.

## Bounds and failure

- Projection retains only the configured cell-byte prefix at a UTF-8 boundary
  while counting the complete projected byte length.
- Once the prefix fills, later syntax cannot replace skipped bytes; retained
  bytes are always a true prefix.
- Parsing rejects malformed parentheses, quotes, empty arguments, invalid map
  arity, and type nesting beyond 64 levels.
- Decoding rejects a collection or aggregate structured value above one million
  nodes. This is a protocol failure below presentation, not partial success.
- All bytes needed for an accepted value are consumed even after its retained
  projection fills. Cancellation remains owned by the official cursor/query.
- Debug and diagnostic contracts reveal kind, bounds, and failure class, never
  cell content.

## Evidence

Unit tests cover recursive/named type grammar, commas inside enum labels,
hostile nesting depth, UTF-8 prefix behavior, and exact original-length
reporting. Testcontainers Rust 0.27.3 runs the same decoder against immutable
ClickHouse 25.8.28.1 and 26.3.17.4 LTS images. Both no-compression and LZ4 paths
prove arrays, nullable tuples, maps, named nested records, binary children, and
truncated structured projections.

External concepts: ClickHouse type grammar and RowBinary container ordering
Public sources: <https://clickhouse.com/docs/sql-reference/data-types/array>, <https://clickhouse.com/docs/sql-reference/data-types/tuple>, <https://clickhouse.com/docs/sql-reference/data-types/map>, <https://clickhouse.com/docs/interfaces/formats/RowBinary>
Implementation source: TableRock-owned parser, bounded projection, and tests
Copied code/assets/text: none
