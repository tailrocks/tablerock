# Native presentation store naming

Date: 2026-08-12

Checkpoint: P3c — retire the bridge-owned model name

## Decision

`BridgeModel` is now `WorkbenchPresentationStore` across production source,
commands, window hosting, tests, and ownership verifiers. Test files and test
classes follow the same name.

The new name states actual ownership: one window-scoped, MainActor-isolated
presentation store consumes a bridge-neutral Feature protocol. It does not own
the concrete UniFFI bridge, generated records, persistence, credential
resolution, or database safety policy.

## Bounds and failure truth

- This was a mechanical symbol and test-file rename. Stored state, method
  bodies, Observation identity, environment injection, commands, and behavior
  did not change.
- Design Lab isolation now forbids both the retired and current production
  store names.
- Source ownership now rejects any return of the misleading `BridgeModel`
  name in app source or App tests.
- Historical evidence retains its original terminology; the current design
  contract now uses the production name.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- Structural and runtime gates passed for multi-window ownership, query tabs,
  and object tabs.
- `scripts/verify-native-source-ownership.sh` passed with the retired-name
  assertion.
- XcodeGen regenerated the renamed test files and `git diff --check` passed.

## Remaining work

Split `WorkbenchPresentationStore` into workflow extensions while keeping one
window-owned observable state object and the same verification matrix.

## Clean-room provenance

This checkpoint changes naming only. TablePro public user-visible organization
informed the confirmed workbench direction; no TablePro source, tests,
comments, bundle internals, branding, assets, copy, or proprietary fixtures
were used.
