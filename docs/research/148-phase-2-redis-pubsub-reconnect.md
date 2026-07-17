# Phase 2 Redis Pub/Sub Reconnect Evidence

Date: 2026-07-17

## Decision

Redis Pub/Sub reconnects and resubscribes only the active read-only channel or
pattern selector. It uses the session's existing bounded attempt count and
exponential delay range. Authentication and protocol failures stop immediately;
connection and timeout failures may retry. Cancellation interrupts connection
setup and backoff. Exhaustion terminates with the last stable failure class.

Resubscription never claims continuous delivery. Redis Pub/Sub is at-most-once,
so messages published while the connection is absent may be lost. After a
successful resubscription, the stream inserts an ordered
`DeliveryDiscontinuity` marker before any post-reconnect message. The shared
page boundary exposes that marker as a zero-row partial page with the same
columns and a dedicated `PageWarning::DeliveryDiscontinuity`. Consumers must
render the gap and may call again with the same row offset because no row was
delivered.

Pre-disconnect queued messages remain before the marker. Queue capacity also
counts the marker; saturation fails as `SubscriptionOverflow` rather than
hiding the gap. Reconnect never replays commands or writes and never creates a
second competing subscription owner.

RESP2 rebuilds a dedicated redis-rs `PubSub` connection and repeats
`SUBSCRIBE`/`PSUBSCRIBE`. RESP3 rebuilds the dedicated multiplexed push
connection and push callback before repeating the command. Both retain binary
selectors, logical database, TLS/authentication client construction, field
bounds, cancellation ownership, and redaction behavior.

Each RESP3 attempt owns an RAII generation token. Every error, cancellation, or
timeout exit deactivates that callback, including the ambiguous case where the
server may have applied `SUBSCRIBE` before the acknowledgement timed out. Only
a fully returned generation is committed; the previous committed generation is
deactivated before another attempt begins.

## Evidence

Testcontainers Rust replaces immutable official Redis 7.4.9 and 8.8.0
containers on the same fixed endpoint. Under RESP2 and RESP3, the real-server
matrix proves automatic channel and pattern resubscription, a zero-row two- or
three-column gap page ordered before the first restored binary message, exact
pattern/channel/payload delivery, and prompt cancellation after the replacement
server is removed again. A blackhole TCP fixture proves every RESP2 and RESP3
connection/subscription attempt times out and the bounded attempt set exhausts.
The core page test proves the new warning bit remains independent from byte-limit
and partial-failure warnings. A generation-guard regression proves abandoned
RESP3 attempts become inactive while committed attempts remain live.

This closes bounded Pub/Sub reconnect/resubscription and visible delivery-gap
truth. TLS Pub/Sub composition is subsequently closed by research 149. DNS endpoint changes, live credential revocation,
strict RESP2 pre-decode allocation bounds, UI presentation, and clean-machine
release evidence remain open.

Context7 selected `/redis-rs/redis-rs`; redis-rs documentation identifies RESP2
dedicated Pub/Sub, RESP3 push senders, and connection-drop classification. The
Testcontainers documentation query reached its service quota, so stop/start API
behavior was verified against pinned testcontainers 0.27.3 source and direct
tests. In-place restart did not restore Testcontainers' macOS port forwarder;
the final fixture replaces the container on one explicitly mapped endpoint.

## Provenance

External concept: Redis at-most-once Pub/Sub and redis-rs connection APIs
Public sources: <https://redis.io/docs/latest/develop/pubsub/> and
<https://docs.rs/redis/1.4.0/redis/aio/struct.PubSub.html>
TableRock requirements: research 06, 10, 14, 20, 30, 31, 32, 53, 145, and 147
Implementation source: TableRock-owned reconnect worker, warning, and page
contracts
Copied code/assets/text: none
