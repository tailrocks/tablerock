# Native fixture projection boundary

Date: 2026-08-12

Checkpoint: P3a — project proof environment before moving presentation state

## Decision

`NativeWorkbenchFixtureConfiguration` is now the sole environment projection
used by `BridgeModel`. The model receives a typed immutable value and no longer
reads process environment while coordinating window state or workflows.

This removes the architectural condition that would have spread Release
fixture symbols when the presentation store leaves `TableRockApp.swift`. It
does not make fixtures production behavior: an ordinary launch projects an
empty value, while the later development-support checkpoint will remove the
environment projection from Release target membership entirely.

## Bounds and failure truth

- Every existing boolean, path, URL, and bounded grid-row fixture key maps to
  one field with the same exact-value semantics.
- Invalid grid counts still disable the performance fixture; positive counts
  retain the existing 10,000-row bound in the model.
- A second-window proof model receives the same immutable fixture value instead
  of rereading global environment.
- Production symbols and allowed debt paths did not grow or spread.
- No bridge, database, safety, persistence, or presentation behavior changed.

## Evidence

- Three focused unit tests prove empty projection, every supported value, and
  exact `"1"` boolean matching.
- `TableRockAppTests`: 24 passed, zero failures.
- Focused quick-filter UI test passed in isolation.
- Structural and runtime gates passed for multi-window ownership, object tabs,
  query tabs, CSV reviewed apply against PostgreSQL, result copy/loaded export/
  streaming export, and the 10,000-row performance fixture.
- `scripts/build-native-app.sh` and
  `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the test target and `git diff --check` passed.

The first unit-test run used the executable target's SwiftPM name
`TableRockApp` instead of its Xcode module name `TableRock`; correcting the
test import restored the green 24-test checkpoint. A full UI-suite run remained
non-green with 21 existing order/host-isolation failures after 459.8 seconds.
The focused test and independent runtime gates passed; full-suite isolation
remains required before final release acceptance.

## Remaining work

Move the fixture-symbol-free model into `WorkbenchPresentationStore.swift`,
then split workflow extensions without changing cancellation or restoration
semantics.

## Clean-room provenance

This checkpoint changes test-support injection only. TablePro public
user-visible organization informed the confirmed workbench direction; no
TablePro source, tests, comments, bundle internals, branding, assets, copy, or
proprietary fixtures were used.
