# Native result-surface extraction

Date: 2026-08-13

Checkpoint: P4x — extract result and inspector composition

## Decision

Result-grid composition, selected-value inspector, relationship continuum,
copy/export menus, and typed value renderers now live in
`ResultSurfaces.swift`.

## Bounds and failure truth

- Quick filtering, visible-row selection mapping, paging, copy/export formats,
  relationship loading/error/empty states, text/JSON/binary/truncation truth,
  and accessibility facts moved unchanged.
- Rust still owns typed cells, stable identity, copy formatting, export bounds,
  structured-value limits, relationship facts, and redaction.
- AppKit use is limited to pasteboard integration; dense result rendering
  remains isolated in `CatalogGrid.swift`.
- No backend, fixture, generated FFI, or Design Lab dependency entered the
  result-surface file.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- `scripts/verify-native-value-inspector.sh` passed typed value structural and
  runtime gates.
- `scripts/verify-native-result-copy.sh` passed shared Rust formatting, native
  copy, loaded export, and full streaming export gates.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 2,331 to 1,685 lines.

## Remaining work

Consolidate environment chrome and separate remaining app-only fixture/support
code. Then enforce the Presentation module and remove Release fixture/scripted-
backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
