# Native Release development-support isolation

Date: 2026-08-13

Checkpoint: P6a — compile-time isolate launch fixtures and scripted backend

## Decision

Deterministic native launch routes and `ScriptedWorkbenchBackend` now compile
only when `TABLEROCK_DEVELOPMENT_SUPPORT` is defined. Xcode enables that
condition for Debug and Test Release, never Release. SwiftPM enables it only
for Debug. Release startup resolves only the live backend and production
Application Support root.

The former `TableRockApp.swift` fixture monolith moved under
`TableRockApp/DevelopmentSupport`; live backend selection now has a separate
composition owner. The source-ownership gate verifies the conditional-build
matrix, development-support guards, route inventory, and absence of fixture
environment routes or the scripted backend outside their quarantine.

## Bounds and failure truth

- Debug/Test deterministic launch routes and unit-test backend behavior remain
  unchanged.
- Release ignores test and fixture environment variables rather than merely
  selecting false runtime flags.
- This is P6a, not final P6: the older presentation-only Row Continuum sample
  neighbor map still produces static fixture markers in Release. It must be
  replaced by a Rust-owned live contract or removed before production
  readiness is claimed.
- The Presentation target extraction remains P5 work.

## Evidence

- `scripts/verify-native-source-ownership.sh` passed.
- `scripts/build-native-app.sh` passed with development support enabled.
- `swift package dump-package --package-path native` passed.
- `swift test --package-path native -c release` passed: 57 tests, 4 live-server
  skips, 0 failures; release configuration tests prove development environment
  values are ignored.
- Focused Xcode Test Release suites passed: TableRockFeatureTests 37/0 and
  TableRockAppTests 24/0.
- Release archive succeeded for arm64 and x86_64.
- Required negative binary scan found no `TABLEROCK_FIXTURE_` or
  `ScriptedWorkbenchBackend` string.
- `scripts/verify-native-query-tabs.sh` passed its structural/runtime fixture
  gate, proving Test/Debug route continuity.
- `git diff --check` passed.

## Clean-room provenance

This checkpoint changes build ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
