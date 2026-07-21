# 581 — Native catalog-refresh XCUITest

Date: 2026-07-21

## Behavior

The canonical app UI suite now connects through the temporary-connection
control, clicks a stable catalog-refresh control, and observes a schema/table
hierarchy in the real AppKit outline. The isolated scripted backend provides
deterministic healthy-session and parent/child catalog facts; a model test
independently proves the health and hierarchy contract.

This is native interaction/presentation proof. Rust real-server adapter and
bridge suites remain authoritative for database catalog semantics.

## Verification

`swiftc -parse` and `git diff --check` pass locally. Full type checking and
real-control execution remain pending on the hosted Xcode 26.6 checkpoint after
push.

## Provenance

Implementation source: TableRock's catalog requirements, injected backend
boundary, and stable accessibility contract.

TablePro influence: broad catalog-navigation workflow only. No source, tests,
identifiers, assets, product text, screenshots, layout measurements, colors,
or key bindings were copied or translated.
