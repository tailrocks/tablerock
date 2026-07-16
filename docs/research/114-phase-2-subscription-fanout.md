# Phase 2 Bounded Subscription Fan-Out

## Decision

Operations no longer own one implicit consumer queue. Each owns a caller-
selected nonzero number of opaque `SubscriptionId` handles, capped at 16, and
each handle owns an independently bounded `OperationEventQueue`.

Subscription IDs are unique across the coordinator. A new subscription supplies
its last delivered sequence:

- an exact authoritative match starts current with an empty queue;
- a behind cursor receives one required resync marker;
- a future cursor fails without creating a handle.

There is no replay buffer and no unbounded catch-up path. Resync consumers reload
authoritative snapshots through the service contract.

## Independent delivery

The coordinator advances its authoritative lifecycle cursor once, then fans the
same immutable event to every queue. The returned `FanoutOutcome` reports exact
subscriber, enqueue, coalescing, and resync counts without payload contents.

Capacity exhaustion affects only the slow queue: it becomes resync while current
subscribers continue receiving/coalescing normally. A consumer addresses both
operation and subscription, so foreign or unknown handles fail closed.

Terminal operations cannot retire while any subscription has queued delivery.
Even drained handles must explicitly unsubscribe before record removal, making
client lifetime and ownership deterministic for both TUI and future UniFFI.

## Evidence

`tablerock-core/tests/service.rs` proves two-subscriber semantic equivalence,
late resync, future and duplicate rejection, finite capacity, unknown-handle
failure, independent slow/fast overflow behavior, pending-event protection, and
explicit unsubscribe before terminal retirement. The canonical opaque-ID suite
also covers `SubscriptionId` wire/text round trips.

This contract derives from TableRock research 10, 14, 30, 31, and 32. No
external product source, tests, identifiers, assets, or protected expression
were used.
