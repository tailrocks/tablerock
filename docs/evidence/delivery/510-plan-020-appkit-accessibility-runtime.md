# Plan 020 — AppKit accessibility runtime tracer

Date: 2026-07-19

## Runtime proof

The earlier accessibility gate inspected source structure only. The app now has
a test-only fixture that renders the actual `NSOutlineView`, `NSTableView`, and
`NSTextView` wrappers together. After AppKit creates the view hierarchy, an
in-process tracer resolves the native controls and requires their exposed
labels plus a complete first-responder round trip:

```text
ACCESSIBILITY_PROOF_PASSED outline=Database_catalog grid=Query_results editor=SQL_editor focus=editor-grid-editor
```

`verify-native-accessibility.sh` builds the strict Swift 6 Release app, launches
this fixture through LaunchServices, waits for the durable proof, and terminates
only its owned process. It still enforces all static semantic, observation,
concurrency, material, and custom-control rules.

This closes the Plan 020 first accessibility/focus path without claiming the
full system VoiceOver, large-content, or multi-window matrix reserved for Plan
021. No assistive-access permission or global accessibility preference change
is needed for the in-process AppKit contract.

## Provenance

TablePro was used only to confirm the broad concept of keyboard-accessible
native database controls. No source, tests, text, screenshots, layouts,
measurements, colors, assets, or key bindings were copied or translated.
