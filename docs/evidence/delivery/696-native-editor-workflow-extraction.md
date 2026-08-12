# Native editor workflow extraction

Date: 2026-08-13

Checkpoint: P3m — extract editor find/replace workflows

## Decision

Find/replace presentation lifecycle, document/selection scope, traversal,
literal and regular-expression replacement, and scope adjustment now live in
`WorkbenchPresentationStore+Editor.swift`.

The editor engine remains independent of SwiftUI rendering. The presentation
store only owns dialog state and selected query-tab projection.

## Bounds and failure truth

- Scope semantics, match selection, replacement counts, zero-width regular
  expression handling, error projection, and status text moved unchanged.
- Private effective-scope helpers remain private to the editor extension. No
  store state visibility changed and no public API was added.
- No fixture symbol, generated FFI type, backend dependency, or Design Lab
  dependency entered the editor extension.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- The focused find/replace behavior test passed document and selection scopes,
  modes, and zero-width regular expressions.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.

## Remaining work

Extract DDL and table-operation safety workflows. Then enforce the Presentation
module and remove Release fixture/scripted-backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
