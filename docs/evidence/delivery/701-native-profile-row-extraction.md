# Native profile-row extraction

Date: 2026-08-13

Checkpoint: P4d — extract connection-row presentation

## Decision

The dense connection row now lives in `ProfileRow.swift`. Engine identity,
target and safety summary, active/environment status, live health, plaintext
warning, and combined accessibility label form one reusable native row surface.

## Bounds and failure truth

- Typography, truncation, engine badge, semantic warnings, live-state
  projection, and accessibility summary moved unchanged.
- The row receives immutable profile and connection-state values. It owns no
  navigation, persistence, secret, session, or backend state.
- No fixture symbol, generated FFI type, backend import, AppKit adapter, or
  Design Lab dependency entered the row file.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- Profile-group structural/runtime gate passed, exercising row projection in
  grouped and active connection states.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 6,469 to 6,365 lines.

## Remaining work

Extract the connections sidebar, workbench surfaces, and narrow AppKit adapters.
Then enforce the Presentation module and remove Release fixture/scripted-backend
membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
