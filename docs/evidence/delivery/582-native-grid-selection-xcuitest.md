# 582 — Native grid-selection XCUITest

Date: 2026-07-21

## Behavior

Each resident native result cell exposes a stable revision-local row/column
automation identifier. The canonical UI suite launches an isolated structured
result with no selection, clicks its real AppKit cell, and requires both the
typed value inspector and JSON tree to appear. The assertion proves selection
drives presentation state; pre-populating an inspector is not accepted.

The fixture contains one bounded immutable result and no database I/O. Rust
page/result contracts remain authoritative for value semantics and ownership.

## Verification

`swiftc -parse` and `git diff --check` pass locally. Full AppKit selection and
XCUITest execution remain pending on the hosted Xcode 26.6 checkpoint after
push.

## Provenance

Implementation source: TableRock's native grid/value-inspector requirements
and stable automation contract.

TablePro influence: broad grid-selection workflow only. No source, tests,
identifiers, assets, product text, screenshots, layout measurements, colors,
or key bindings were copied or translated.
