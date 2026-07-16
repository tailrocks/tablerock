# Phase 2 operation-to-driver routing

Date: 2026-07-17

`DriverRuntime` is the engine-owned operation execution seam. It maps bounded
core `OperationId` values to owned Tokio tasks, type-erased driver sessions,
single-slot cancel channels, latest-state stop signals, and bounded event
channels. It rejects invalid
limits, duplicate identity, capacity overflow, and unknown joins.

Each task owns stream start, sequential immutable page production, checked row
offsets, cancellation dispatch, and consuming session shutdown. Control remains
responsive when event delivery is backpressured: the task stops polling the
database stream while one bounded local delivery queue is pending, but still
selects cancellation and stop independently. Duplicate cancellation is
idempotent. Runtime shutdown requests client stop and does not wait for a slow
event consumer. Consuming session shutdown completes before the authoritative
terminal event/exit, so a late connection failure cannot follow a false
completion.

The runtime deliberately does not own lifecycle truth. Unknown operations are
reported as unknown, unsupported adapters remain unsupported, and request
delivery is not reported as server-confirmed cancellation. `DriverTaskExit` is
the authoritative task observation (`Completed`, `ClientStopped`, or safe
`Failed`); the core coordinator decides the legal terminal lifecycle edge.

Contract tests prove capacity, duplicate identity, unknown cancellation,
cancellation while a one-event output channel is backpressured, task joining,
client-stop shutdown, and consuming session shutdown. Real Testcontainers
suites continue to prove each driver boundary.

This checkpoint introduces no external-product influence. Sources are the
approved TableRock architecture and shared-client contract.
