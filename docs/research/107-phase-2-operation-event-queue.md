# Phase 2 Bounded Operation Event Queue

## Structural correction

The architecture permits progress coalescing, but the original consumer cursor
rejected every event-sequence gap. Those rules could not both hold: replacing a
pending progress event preserves cumulative truth while advancing the delivered
sequence.

The corrected contract accepts a gap only for cumulative progress carrying the
exact `coalesced_after` sequence matching the consumer cursor. The queue writes
that proof while replacing consecutive progress; ordinary producers set no
proof. Required phase, terminal, failure, and resync events still require
contiguous history. Progress must also retain the same revision, occur in an
active phase, and never regress rows or bytes.

## Queue contract

`OperationEventQueue` owns delivery for one operation identity:

- capacity is caller-selected, nonzero, and capped at 4,096 events;
- only consecutive pending progress events coalesce;
- coalescing records the exact pre-range sequence consumed by cursor validation;
- foreign, stale, and duplicate events are rejected;
- a producer gap or capacity exhaustion clears pending delivery and inserts one
  required `ResyncRequired` marker;
- the marker records the last event actually delivered to the consumer;
- diagnostics expose identity, capacity, sequence, and counts only.

The application service will compose these queues into bounded subscriptions.
It must rebuild authoritative snapshots after resync and must not treat the
marker as a terminal database outcome.

## Evidence

`tablerock-core/tests/operation.rs` proves cumulative progress-gap acceptance,
consecutive coalescing, exact queue capacity, overflow and producer-gap resync,
last-delivered cursor capture, and rejection of invalid capacity, foreign
identity, stale delivery, and duplicates. Existing lifecycle tests continue to
prove cancellation truth and legal phase edges.

This contract is derived from TableRock research 10, 14, 30, 31, and 32. No
external product source, protected expression, assets, or product text were
used.
