# Phase 2 Redis Pattern Subscription Evidence

Date: 2026-07-17

## Decision

Pattern subscriptions are a distinct bounded stream contract, not a nullable
extension of channel pages. Channel subscriptions retain two binary columns:
`channel` and `payload`. Pattern subscriptions return three binary columns:
`pattern`, `channel`, and `payload`. The requested pattern and the delivered
channel therefore remain independently observable without decoding or text
conversion.

The shared adapter carries an explicit `RedisSubscriptionKind` and one bounded
binary selector. RESP2 uses redis-rs `PSUBSCRIBE`/`PUNSUBSCRIBE`; RESP3 uses the
same commands on a dedicated push connection and decodes `PMessage` through
redis-rs's message abstraction. A channel frame on a pattern stream, or a
pattern frame on a channel stream, is a protocol failure.

Pattern streams reuse the generation-safe long-operation ownership, bounded
message queue, page/cell/arena limits, cancellation, drop teardown, redaction,
and explicit overflow behavior established by research 145. The selector must
fit the configured cell bound before network I/O. Every decoded field is
truncated with its original byte length before entering the count-bounded queue,
so TableRock-owned retained payload memory is bounded by queue capacity, column
count, and the per-field byte limit. Page arena limits can truncate it further
without losing the original length. A pattern page requires capacity for all
three columns before network I/O.

Automatic reconnect is deliberately absent. Redis Pub/Sub is at-most-once and
a disconnected interval can lose messages; a future reconnect contract must
surface that delivery discontinuity before resubscribing rather than silently
presenting a continuous stream.

## Evidence

Testcontainers Rust runs immutable official Redis 7.4.9 and 8.8.0 images under
RESP2 and RESP3. The real-server matrix proves binary pattern, matching channel,
and payload preservation through the object-safe driver boundary; three-column
and selector-limit rejection before I/O; oversized payload truncation before
queue retention with exact original-length metadata; explicit client-stop
cancellation; and `PUBSUB NUMPAT` returning zero after teardown. Existing channel delivery,
ordinary-command isolation, overflow, service cancellation, and generation
ownership evidence continues to pass in the same matrix.

This closes pattern subscription transport and paging. Reconnect/resubscription
with visible delivery gaps, server restart and DNS races, TLS Pub/Sub composition,
strict RESP2 pre-decode allocation bounds, UI presentation, and clean-machine
release evidence remain open.

Context7 selected `/redis-rs/redis-rs`; behavior was cross-checked against the
pinned redis-rs 1.4.0 source and official Redis command references.

## Provenance

External concept: Redis pattern Pub/Sub and redis-rs binary message decoding
Public sources: <https://docs.rs/redis/1.4.0/redis/aio/struct.PubSub.html>,
<https://redis.io/docs/latest/commands/psubscribe/>, and
<https://redis.io/docs/latest/commands/punsubscribe/>
TableRock requirements: research 03, 06, 10, 14, 20, 30, 31, 32, 53, and 145
Implementation source: TableRock-owned bounded stream, page, and ownership
contracts
Copied code/assets/text: none
