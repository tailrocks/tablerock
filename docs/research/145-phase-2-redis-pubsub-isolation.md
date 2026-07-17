# Phase 2 Redis Pub/Sub Isolation Evidence

Date: 2026-07-17

## Decision

Redis Pub/Sub is a long-lived, session-owned operation. It never changes the
shared command connection into subscriber mode. RESP2 uses redis-rs's dedicated
async Pub/Sub connection. RESP3 uses a dedicated multiplexed connection with a
nonblocking push sender. Both paths retain the session's protocol, logical
database, TLS, client identity, and ACL construction through the same client.

The public stream returns immutable two-column `channel`/`payload` binary pages.
Page row, arena, cell, and queued-message limits are mandatory. The retained
queue is bounded to 1..=4096 messages. A full queue terminates with the stable
`SubscriptionOverflow` resource-limit class; TableRock never silently drops a
message. Research 147 subsequently bounds decoded fields before they enter this
queue and bounds selectors before I/O. Redis-rs's RESP2 decoder has an internal unbounded handoff before the
TableRock queue, so a strict pre-decode transport allocation cap remains open
and is not claimed here.

One atomic long-operation gate plus generation-safe registries owns each active
subscription or blocking command. Those operation kinds exclude one another,
while ordinary multiplexed commands stay usable. Setup awaits race cancellation.
Cancel and drop wake the owning worker, attempt `UNSUBSCRIBE` within the response
timeout, then drop the dedicated connection and release only the matching
generation. Explicit cancellation terminates as `ClientCancelled`. This is truthful
client-stop evidence, never server-confirmed query cancellation.

## Evidence

Testcontainers Rust runs immutable official Redis 7.4.9 and 8.8.0 images. Under
RESP2 and RESP3 the real-server matrix proves binary channel and payload
preservation, ordinary-command isolation, bounded option rejection, explicit
overflow, object-safe service cancellation terminating as client-stop, active
drop/replacement generation behavior, and `PUBSUB NUMSUB` returning zero after
unsubscribe.

This closes the bounded Pub/Sub isolation tracer. Pattern subscriptions are
subsequently closed by research 147. Reconnect/resubscription and same-endpoint
server replacement are subsequently closed by research 148. DNS races, strict RESP2
pre-decode allocation bounds, TLS Pub/Sub composition evidence, UI presentation,
and clean-machine release evidence remain open.

Context7 selected the official `/redis-rs/redis-rs` documentation. API behavior
was cross-checked against pinned redis-rs 1.4.0 source.

## Provenance

External concept: Redis Pub/Sub and redis-rs async push delivery
Public sources: <https://docs.rs/redis/1.4.0/redis/aio/struct.PubSub.html>,
<https://docs.rs/redis/1.4.0/redis/struct.AsyncConnectionConfig.html>, and
<https://redis.io/docs/latest/develop/pubsub/>
TableRock requirements: research 03, 06, 10, 14, 20, 30, 31, 32, and 53
Implementation source: TableRock-owned bounded stream and ownership contracts
Copied code/assets/text: none
