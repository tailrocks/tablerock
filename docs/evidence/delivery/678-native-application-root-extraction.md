# Native application root extraction

Date: 2026-08-12

Checkpoint: P2c — extract application entry and window hosting

## Decision

The SwiftUI `@main` entry, `WindowGroup`, workbench window host, restoration,
settings composition, and narrow AppKit window configuration now live in
`native/Sources/TableRockApp/ApplicationRoot.swift`.

An immutable `NativeLaunchConfiguration` projects the existing deterministic
test environment into typed launch surface and multi-window facts. Its
environment parsing remains in the existing monolith debt owner; the extracted
production root consumes typed state and contains no fixture environment key.
`NativeApplicationModel` similarly owns the resolved launch and appearance
configuration for process lifetime.

## Bounds and failure truth

- Window identity, tabbing, restoration, minimum size, URL routing, shared
  backend/per-window store ownership, settings, commands, and fixture behavior
  did not change.
- Internal visibility widened only for app-target views/configuration invoked
  across the new file boundary. No public API was added.
- Existing fixture environment parsing remains temporary Release debt. It did
  not spread to `ApplicationRoot.swift` or `ApplicationRuntime.swift`.
- Presentation surfaces and fixture implementations remain in the monolith for
  later checkpoints.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `scripts/verify-native-accessibility.sh` passed structural and runtime proof.
- `scripts/verify-native-profile-editor.sh` passed structural and runtime proof.
- `scripts/verify-native-multi-window.sh` passed structural and runtime proof.
- `TableRockAppTests`: 21 passed, zero failures.
- `scripts/verify-native-source-ownership.sh` passed; existing Release fixture
  debt neither gained symbols nor spread files.
- XcodeGen regenerated the app target with `ApplicationRoot.swift`.
- `git diff --check` passed.

## Remaining work

Extract commands and presentation drafts, then isolate live bridge ownership
and conversions. Scripted/test routes remain scheduled for development-only
isolation before final Release contraction.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
