# 552 — Live WorkbenchBackend protocol isolation

Date: 2026-07-21

## Decision

`BridgeModel` no longer stores or constructs the concrete UniFFI client. It
depends on the actor-bound `WorkbenchBackend` protocol across profile, session,
catalog, query, result, import/export, structure, Redis, history, and review
operations. `LiveWorkbenchBackend` is the sole owner of `TableRockBridge`, its
event cursor, synchronous pump, and off-main PageV1 decode.

The actor requirement makes every backend implementation `Sendable`, retains
serialized event-cursor ownership, and preserves identity sharing across native
windows without exposing UniFFI ownership to presentation state.

## Verification

```text
./scripts/build-native-app.sh
```

The strict Swift 6 Release app builds and links after existential backend
substitution.

## Remaining checkpoint 12 work

- replace generated bridge records in the protocol with application-owned
  immutable DTOs;
- add `ScriptedWorkbenchBackend` and the named deterministic scenario matrix;
- prove model deallocation during active work in the canonical test target.

No external product influenced this dependency-inversion repair.
