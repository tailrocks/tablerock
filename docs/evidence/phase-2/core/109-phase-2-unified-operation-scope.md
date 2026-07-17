# Phase 2 Unified Operation Scope

## Structural correction

The original `OperationIdentity` required a profile/session/context
`OperationScope`. That prevented valid application, profile, and session
commands—such as shutdown, connect, and disconnect—from entering the same
operation lifecycle.

`OperationIdentity` now owns the existing typed `CommandScope`. The four legal
forms are application, profile, session, and context. This is one canonical
scope hierarchy: no optional identifiers, sentinel IDs, context-only exception,
or parallel service identity remains.

## Safety and compatibility

Every event still carries the exact identity of its originating command.
Foreign-event rejection and bounded per-operation queues therefore work at all
service levels. This is a forward-only API correction; consumers must wrap live
context scope as `CommandScope::Context` and must not retain the old signature.

## Evidence

The public operation contract test constructs and round-trips every scope form.
Existing lifecycle, cursor, coalescing, overflow, and resync tests pass through
the unified identity.

This decision derives from TableRock's fixed command/event and shared-service
requirements. No external product implementation or protected expression was
used.
