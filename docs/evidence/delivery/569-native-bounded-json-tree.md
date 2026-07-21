# 569 — Native bounded JSON tree

Date: 2026-07-21

## Decision

The native value inspector now presents structured JSON as a deterministic
flattened tree while retaining its text and hexadecimal views. Parsing lives in
the application-owned `TableRockFeature` boundary, not the SwiftUI view. Object
keys sort lexically; array order remains exact; scalar/null kinds stay explicit.

The parser rejects input above 64 KiB before JSON decoding and fails closed at
1,024 projected nodes or 64 levels. Invalid JSON produces no tree rather than a
misleading partial structure. The existing Rust page decoder remains the owner
of value kind, truncation, and raw bytes.

The inspector and tree have stable accessibility identifiers. A scripted
XCUITest opens the structured-value fixture and requires both surfaces.

## Verification

```text
swiftc -parse-as-library -emit-module native/Sources/TableRockFeature/StructuredValueTree.swift
PATH=/Users/donbeave/.cargo/bin:$PATH ./scripts/build-native-app.sh
PATH=/Users/donbeave/.cargo/bin:$PATH ./scripts/verify-native-value-inspector.sh
mise exec github:yonaskolb/XcodeGen@2.46.0 -- xcodegen generate --spec native/App/project.yml
```

Results: strict Swift feature compilation, the complete direct native build,
and the structural/runtime inspector gate pass. The runtime fixture proves
AppKit selection plus text/hex projection and exact JSON-tree model rows. The
hosted canonical Xcode plan owns the two parser tests and the user-operable UI
test; hosted result is pending for this checkpoint.

Local `swift test` cannot run because the selected Command Line Tools SDK lacks
the XCTest module used by pre-existing feature tests. No local XCTest claim is
made.

## Remaining boundary

The native inspector still lacks stale-value and staged/original projections.
The current tree is read-only and fully expanded; expand/collapse state is not
claimed. Hosted XCTest proof remains required before closing this checkpoint's
test-plan gate.

## Provenance

Implementation source: TableRock-owned typed page/value requirements, Swift
feature boundary, and direct tests.

TablePro influence: its public product demonstrates the broad structured-value
inspection workflow only. No layout, product text, colors, measurements,
assets, identifiers, key bindings, source, tests, or screenshots were copied
or translated.
