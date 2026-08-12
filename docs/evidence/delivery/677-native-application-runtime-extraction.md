# Native application runtime extraction

Date: 2026-08-12

Checkpoint: P2b — extract backend composition runtime

## Decision

`NativeApplicationModel` now lives in
`native/Sources/TableRockApp/ApplicationRuntime.swift`. This application-only
composition object resolves paths, constructs capability ports, requests the
configured backend from the current backend factory, and shares one backend
across independently owned window presentation stores.

The concrete live/scripted factory stays beside those implementations for this
checkpoint. This preserves the P1 rule that existing Release fixture debt may
not spread into a new file. Moving the live facade and isolating scripted
support are later explicit checkpoints.

## Bounds and failure truth

- Backend selection, test-mode confinement, shared-backend/multi-window
  behavior, startup error truth, and data-root resolution did not change.
- The live and scripted backend implementations and their concrete factory
  remain in the monolith for the next extraction checkpoints.
- SwiftUI entry/window hosting and presentation state remain separate pending
  responsibilities; this checkpoint does not claim P2 complete.

## Evidence

- `scripts/build-native-app.sh` passed under strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 21 passed, zero failures, including shared backend with
  independent window presentation state.
- XcodeGen regenerated the project with `ApplicationRuntime.swift` in the app
  target.
- `scripts/verify-native-source-ownership.sh` and `git diff --check` passed.

## Remaining work

Extract application entry/window hosting and commands, then move live bridge
facade/conversions behind their final bridge ownership boundary.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
