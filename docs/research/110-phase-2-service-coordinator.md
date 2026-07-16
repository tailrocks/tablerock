# Phase 2 Application Service Coordinator

## Decision

`ServiceCoordinator` is the std-only authoritative owner for scoped aggregate
revisions, operation lifecycle, and delivery state shared by future in-process
and UniFFI adapters. It retains caller-selected nonzero capacities capped at
16,384 scopes and 4,096 operations, and each operation owns the already-bounded
event queue.

Scopes register parent-first as application, profile, session, and context.
Application scope always exists. Registration rejects duplicates, missing
parents, and capacity overflow. Revision advance is monotonic and compare-and-
swap guarded. Removal rejects application scope, registered children, and any
resident operation inside the scope subtree.

Submission first requires a registered scope and an exact current aggregate
revision. It then binds the externally generated opaque operation ID to the command's
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

Every operation now owns a finite set of opaque subscription handles and one
bounded queue per handle. Fan-out is independent: progress coalescing or resync
for one subscriber does not alter another queue. Late subscribers receive
resync unless their cursor exactly matches the authoritative sequence. Future
cursors fail. Terminal retirement requires all queues drained and all handles
explicitly removed.

Driver task ownership remains later Phase 2 work; it must compose this
coordinator rather than duplicate its scope, revision, subscription, or
lifecycle state machines.

## Evidence

`tablerock-core/tests/service.rs` proves finite scope/operation limits,
parent-first registration, duplicate and capacity rejection, monotonic compare-
and-swap revision advance, stale submission rejection, safe hierarchical
removal, operation/request uniqueness, live-parent containment, stale in-flight
progress rejection, legal lifecycle, exact cancellation outcomes, pending-event retirement,
cancel-active draining, stopped-state rejection, and no invented outcomes.
Multi-subscriber tests additionally prove independent delivery, late resync,
future/duplicate/unknown handle rejection, finite subscriber capacity, and
slow-consumer overflow isolation.

This contract derives from TableRock research 10, 14, 30, 31, and 32. No
external product source, tests, identifiers, assets, or protected expression
were used.
