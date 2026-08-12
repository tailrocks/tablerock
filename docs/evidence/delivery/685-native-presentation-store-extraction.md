# Native presentation store extraction

Date: 2026-08-12

Checkpoint: P3b — extract the window presentation store from the monolith

## Decision

The complete `@MainActor` Observation model now lives in
`native/Sources/TableRockApp/WorkbenchPresentationStore.swift`. Its temporary
type name remains `BridgeModel` so this checkpoint changes one ownership
boundary only; the mechanical rename and workflow split follow after call-site
proof.

`TableRockApp.swift` fell from 11,131 to 7,190 lines. The new 3,951-line store
contains no Release fixture symbol and no scripted-backend implementation
reference. It still consumes a typed fixture configuration until development
support is removed from Release membership.

## Bounds and failure truth

- Stored state, computed projections, method bodies, actor isolation, task
  ownership, call order, and cancellation/restoration behavior moved unchanged.
- Cross-file audit helpers, formatters, relation-continuum fixtures, and native
  find/replace helpers became module-internal only where the store consumes
  them. No public API was added.
- One use of the scripted backend's private error was replaced by a
  store-private error with the same `unavailable("stream-export-session")`
  failure description, removing an implementation coupling without changing
  the surfaced failure text.
- Bridge and generated FFI ownership did not move back into presentation.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `swift test --package-path native -c release` built the app and passed 56
  XCTest cases with four configured live-server skips plus 24 Swift Testing
  cases with zero failures.
- `TableRockAppTests`: 24 passed, zero failures.
- Structural and runtime gates passed for query tabs, object tabs, history,
  saved queries, and SQL files.
- `scripts/verify-native-source-ownership.sh` passed and confirmed no fixture
  debt in the new store file.
- XcodeGen regenerated the app target and `git diff --check` passed.

The first direct compile exposed 19 former file-private collaborators. Their
minimum visibility was corrected to module-internal, except the scripted error
coupling, which was removed. The same direct build then passed.

## Remaining work

Rename `BridgeModel` mechanically to `WorkbenchPresentationStore`, then split
workflow extensions by connection, catalog, navigation, tabs, queries,
transfers, administration, and restoration while retaining this proof set.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
