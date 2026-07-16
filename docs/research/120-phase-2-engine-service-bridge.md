# Phase 2 engine service bridge

Date: 2026-07-17

## Decision

`EngineService` is the in-process application-service seam between both future
clients and the driver runtime. It owns one core `ServiceCoordinator`, one
`DriverRuntime`, and bounded per-operation runtime receivers. Presentation does
not call a driver directly.

Core remains the sole lifecycle authority. Runtime observations map as follows:

| Runtime fact | Core transition |
|---|---|
| started while queued | `Queued -> Running` |
| first immutable page | `Running -> Streaming` plus cumulative row/payload bytes |
| completed | `Completed`, or `CompletedBeforeCancel` after a cancel request |
| client stopped | `ClientStopped` only after a cancel request |
| safe adapter failure | `Failed` |

An immediate cancellation can arrive before the queued `Started` event. The
bridge retains `CancelRequested` and never regresses it to `Running`. A cancel
dispatch remains only request-delivery evidence; it is not a terminal outcome.
The terminal runtime event must equal the authoritative joined task exit or the
bridge fails closed with `TerminalMismatch`.
Closed terminal delivery, task panic, or terminal/join disagreement removes the
runtime receiver and transitions the core record to `Unknown`; it cannot leave
shutdown draining forever on a ghost active operation.

Core submission rejection consumes the supplied session through asynchronous
shutdown and preserves a separately redacted cleanup error. Runtime spawn
rejection marks the already-submitted queued core operation failed. Bounded
subscription, drain, unsubscribe, and retirement semantics remain core-owned;
the bridge exposes subscription delivery and retirement without duplicating
state.

## Evidence

- Contract tests prove queued/running/streaming/completed mapping, immutable page
  delivery, cumulative progress, immediate-cancel non-regression,
  completed-before-cancel truth, and rejected-submission session shutdown.
- PostgreSQL 18.4 Testcontainers now executes its bounded stream through
  `EngineService`, not directly through the runtime.
- Driver/session/client types remain below the service; outputs are core IDs,
  outcomes, events, and immutable bounded pages.

This checkpoint uses TableRock-owned requirements, existing adapters, and direct
tests. No external-product source, identifiers, product text, assets, or
protected expression were used.
