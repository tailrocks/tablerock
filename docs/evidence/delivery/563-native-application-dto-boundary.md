# 563 — Native application DTO boundary

Date: 2026-07-21

## Decision

Plan 021 checkpoint 12 now owns `WorkbenchBackend` and every value crossing it
inside `TableRockFeature`. The records are immutable, `Sendable` application
facts. Generated UniFFI records no longer appear in the protocol or
presentation model.

`LiveWorkbenchBackend` remains the sole `TableRockBridge` owner. Its adjacent
conversion adapter performs explicit generated-record translation in both
directions. The scripted actor conforms without importing the generated bridge.
Mutable connection editing is isolated in `ProfileEditorDraft`; save creates a
new immutable `WorkbenchProfileDraft`. Result pagination likewise creates a new
immutable `WorkbenchTable` instead of mutating a backend value.

## Verification

```text
PATH=/Users/donbeave/.cargo/bin:$PATH ./scripts/build-native-app.sh
./scripts/verify-native-accessibility.sh
```

Strict Swift 6 Release compilation, app bundling, signing, launch, and runtime
accessibility verification pass locally. Hosted XCTest and universal artifact
verification follow through `.github/workflows/native.yml` after this commit.

## Checkpoint 12 closure

Canonical application-model tests now cover scripted scenario selection,
close negotiation, restoration corruption, multi-window ownership, active-work
deallocation, and semantic cancellation publication. The hosted Xcode proof is
recorded in [evidence 564](564-native-xcode-checkpoint.md).

This is dependency inversion, not visual design. TablePro influenced no
expression. Copied source, tests, identifiers, assets, product text,
screenshots, layout measurements, colors, or key bindings: none.
