# 553 — Scripted WorkbenchBackend failure seams

Date: 2026-07-21

## Decision

`AppConfiguration`'s once-parsed scripted backend mode now installs a real
`ScriptedWorkbenchBackend` instead of rejecting startup. Protocol defaults fail
closed for unstaged operations. Named deterministic behaviors cover:

- success and connection/authentication failure;
- slow work that terminates only after explicit cancel;
- stale result revision and stale event;
- cursor resync and mismatched next-page columns;
- history failure after page delivery.

The scripted actor owns cancellation state and returns only immutable bridge
facts. It performs no filesystem, persistence, network, Keychain, panel, or
pasteboard access. Production continues to select `LiveWorkbenchBackend`.

## Verification

```text
./scripts/build-native-app.sh
```

Strict Swift 6 Release compilation proves every scripted/default method
conforms to the same actor-bound backend surface used by `BridgeModel`.

## Remaining checkpoint 12 work

- application-owned DTO replacement for generated bridge records;
- canonical model tests for running/dirty close, restoration corruption,
  multi-window ownership, and deallocation during active work;
- behavioral assertions for every scripted scenario under full Xcode.

No external product influenced this deterministic-test seam.
