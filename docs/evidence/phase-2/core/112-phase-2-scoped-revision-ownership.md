# Phase 2 Scoped Revision Ownership

## Decision

The application-service coordinator now owns the authoritative revision for
every registered command scope. Its caller-selected scope capacity is nonzero,
includes the required application scope, and is capped at 16,384.

Scopes form one strict hierarchy:

```text
Application
  Profile
    Session
      Context
```

Registration requires the exact parent to exist and rejects duplicates or
capacity overflow. This prevents orphan sessions/contexts and removes any need
for optional identity components.

## Revision gate

`advance_scope` uses compare-and-swap semantics: the supplied revision must be
current, counter overflow fails closed, and success returns the exact next
revision. `submit` requires a known scope and exact equality between
`CommandEnvelope::expected_revision` and the authoritative current revision.
Stale and future commands therefore fail before operation capacity, event
queues, cancellation state, or drivers can change.

Lifecycle and terminal truth remain recordable after a scope changes, because
disconnect/cancellation outcomes cannot be erased. Cumulative progress payloads
from an operation dispatched at an older revision are rejected before sequence
or queue state changes.

Scope removal is leaf-first. Application scope cannot be removed; a scope with
registered descendants or any resident operation in its subtree remains owned.
Terminal operations must finish delivery and retire before their scope can
disappear.

## Evidence

`tablerock-core/tests/service.rs` proves parent-first registration, duplicate
and finite-capacity rejection, monotonic advance, stale compare-and-swap and
submission rejection, registered revision lookup, child/in-use removal guards,
stale in-flight progress rejection, and complete leaf-first cleanup after
terminal delivery.

This contract derives from TableRock research 10, 14, 30, 31, and 32. No
external product source, identifiers, tests, assets, or protected expression
were used.
