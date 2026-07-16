# Phase 2 Application Service Coordinator

## Decision

`ServiceCoordinator` is the std-only authoritative owner for operation
lifecycle and delivery state shared by future in-process and UniFFI adapters.
It retains a caller-selected nonzero operation capacity capped at 4,096, and
each operation owns the already-bounded event queue.

Submission binds the externally generated opaque operation ID to the command's
request ID and canonical typed scope. Active operation and request IDs are
unique. A child requires a live parent, and its scope must remain inside the
parent's application/profile/session/context boundary. Missing, terminal, or
scope-incompatible parents fail before state changes.

## Lifecycle and cancellation

The coordinator alone constructs phase/progress events and advances the
authoritative cursor before delivery. It rejects illegal edges and regressing
progress. Cancellation changes queued, running, or streaming work only to
`CancelRequested`; repeated and terminal requests have explicit outcomes.
Drivers must later report the observed terminal truth.

Terminal operations remain resident until every queued event is consumed.
Retirement of active operations or operations with pending delivery fails, so
capacity reclamation cannot erase lifecycle truth.

## Shutdown

Graceful shutdown rejects new submissions and drains active work. Cancel-active
shutdown additionally requests cancellation for each active operation but does
not call the service stopped. The coordinator reaches `Stopped` only after all
operations carry legal terminal outcomes. Existing terminal records may remain
available for delivery and retirement.

The next enclosing application-state checkpoint must own aggregate revisions
and validate `CommandEnvelope::expected_revision` before submission. Driver task
ownership and bounded multi-subscriber fan-out also remain later Phase 2 work;
they must compose this coordinator rather than duplicate its state machine.

## Evidence

`tablerock-core/tests/service.rs` proves finite limits, operation/request
uniqueness, capacity, live-parent and scope containment, legal lifecycle and
cumulative progress, exact cancellation outcomes, pending-event retirement,
cancel-active draining, stopped-state submission rejection, and absence of
invented terminal outcomes.

This contract derives from TableRock research 10, 14, 30, 31, and 32. No
external product source, tests, identifiers, assets, or protected expression
were used.
