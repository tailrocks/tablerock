# Phase 2 ClickHouse RowBinary Foundation

## Decision

TableRock now uses the official `ClickHouse/clickhouse-rs` client 0.15.1 through
one adapter. The dependency enables LZ4 plus rustls with native roots and the
ring provider. Database-client types remain private to `tablerock-engine`.

Arbitrary result metadata comes from `RowBinaryWithNamesAndTypes`. The adapter
parses the streamed column count, names, and exact ClickHouse type strings,
then immediately emits owned immutable `ResultPage` values. This checkpoint
decodes `UInt64`, `Int64`, `Float64`, `String`, `FixedString(N)`, and recursive
`Nullable(T)`. Unsupported types fail explicitly before row decoding; later
Phase 2 checkpoints add the remaining complex-type matrix without carrying a
second result path.

## Bounds and failure

- Column count, metadata text, rows, page arena, and each cell are bounded.
- Oversized variable values are consumed incrementally and retained only to the
  allowed prefix; no full-result collection occurs.
- The official cancellation-safe `BytesCursor::next()` chunk API is the sole
  response source. TableRock does not implement HTTP or compression.
- Late cursor errors fail the stream. No partial page is presented as success.
- Invalid LEB128, nullable markers, metadata, or truncated rows fail closed as
  protocol errors.
- Each query carries a caller-owned bounded query ID. Cancellation-outcome and
  server-observation work remains an explicit Phase 2 blocker.
- Adapter errors contain no query text, credentials, or cell values.

## Real-server evidence

The Testcontainers contract runs both uncompressed and LZ4 responses against:

- ClickHouse `25.8.28.1-jammy` OCI index digest
  `sha256:ea72c2ca1487386451e43525f7e5455811b62095914d8dd4775b1cda6c09d2e3`;
- ClickHouse `26.3.17.4-jammy` OCI index digest
  `sha256:158dcce6f6fdc59309650aad6b79484abf4eed07d4e0bdba31d732e64b5a25fb`.

It proves self-described columns, bounded multi-page delivery, signed and
unsigned integers, exact float bits, nullable text, binary fixed strings, final
delivery, and end-of-stream behavior.

## Remaining Phase 2 proof

Complex nested/array/tuple/map, low-cardinality, decimals, 128/256-bit integers,
dates/times, UUID/IP, enums, aggregate-state fallback, TLS fixtures, progress,
server-observed cancellation, late-error injection, inserts, and mutations are
not yet claimed. They remain ledger blockers.

External concepts: RowBinaryWithNamesAndTypes, ClickHouse HTTP compression, OCI image pinning
Public sources: <https://github.com/ClickHouse/clickhouse-rs/tree/v0.15.1>, <https://clickhouse.com/docs/interfaces/formats/RowBinaryWithNamesAndTypes>, <https://hub.docker.com/_/clickhouse>
Implementation source: official client source/docs, ClickHouse format docs, and TableRock-owned adapter/tests
Copied code/assets/text: none
