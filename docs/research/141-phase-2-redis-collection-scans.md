# Phase 2 Redis Collection SCAN Evidence

Date: 2026-07-17

## Decision

Rust core exposes one bounded `RedisCollectionStream` for Redis hashes, sets,
and sorted sets. `RedisCollectionScanKind` selects `HSCAN`, `SSCAN`, or `ZSCAN`;
redis-rs connection, cursor, and reply types remain private to the adapter. The
same stream crosses the object-safe `DriverPageStream` boundary as other engine
results.

Pages are immutable and report an unknown total. Hash rows contain binary
`field` and `value` cells. Set rows contain one binary `member`. Sorted-set rows
contain a binary `member` and an exact IEEE-754 `double` score. Raw key, field,
value, and member bytes never require UTF-8. Request debug output records only
key length.

Every request bounds rows, columns, column metadata bytes, arena bytes,
per-cell bytes, Redis `COUNT`, accepted decoded batch entries/bytes, and client
scan rounds. Sorted-set validation reserves eight score bytes per possible page
row before dispatch. Truncation produces the existing byte-limit warning. An
initial empty collection emits one zero-row final page with metadata; a later
empty terminal cursor batch emits no redundant page.

Redis SCAN-family iteration is read-only but not a snapshot. Concurrent changes
may cause duplicates or omissions, so no stable total, progress percentage, or
deduplication promise exists. Exhausting the explicit round budget is a bounded
query failure. Transport/server rejection is the message-free command failure.
An accepted-batch bound violation is the distinct message-free adapter
`ResourceLimit` class, not a decode or server-command failure.
Dropping the stream stops further client requests; it does not claim a
server-confirmed cancellation because each scan round is an ordinary finite
command.

## Evidence

Testcontainers Rust 0.27.3 runs immutable official Redis 7.4.9 and 8.8.0
images. Under RESP2 and RESP3, the shared matrix proves:

- HSCAN preserves binary fields and values through the object-safe driver seam;
- SSCAN preserves binary members in one-column bounded pages;
- ZSCAN preserves binary members and exact positive and negative scores;
- page size one continues across a buffered server reply without redundant
  zero-row terminal pages;
- missing HSCAN, SSCAN, and ZSCAN targets emit one typed empty final page;
- an oversized decoded server batch is rejected before entries enter retained
  pending state;
- shape-specific column and arena limits reject impossible requests; and
- sorted-set member truncation cannot consume score storage.

redis-rs fully decodes one command reply before TableRock can inspect its size.
TableRock now bounds all retained pending state, but a strict pre-decode
transport allocation cap is not available through the selected client and
remains an explicit memory-hardening gate. `COUNT` is only a hint; the adapter
does not misrepresent it as that transport cap.

This closes the functional Phase 2 SCAN-family breadth tracer. It does not
close the strict transport-memory gate, complete Redis value views, mutation
review, TLS/authentication, Pub/Sub, timeout/reconnect, the TUI, or native
presentation. Concurrent mutation races are subsequently closed by
[research 142](142-phase-2-redis-scan-mutation-races.md).

Context7 was attempted first and reported its monthly quota exhausted. The
redis-rs query conversion was verified against exact pinned 1.4.0 source.
Official Redis command documentation defines the cursor, `COUNT`, reply-shape,
and concurrent-iteration behavior used here.

## Provenance

External concept: Redis incremental collection iteration  
Public sources: <https://redis.io/docs/latest/commands/hscan/>,
<https://redis.io/docs/latest/commands/sscan/>,
<https://redis.io/docs/latest/commands/zscan/>, and
<https://docs.rs/redis/1.4.0>  
TableRock requirements: research 03, 06, 10, 14, 20, 30, 31, 32, and 90  
Implementation source: TableRock-owned adapter, bounded page contract, and
independent Testcontainers fixtures  
Copied code/assets/text: none
