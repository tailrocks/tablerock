# Native paging scroll gesture

Date: 2026-07-22

## Failure

Native checkpoint run 29851685134 passed Rust bridge generation, Swift tests,
and universal XCFramework construction, then failed compiling XCUITest because
`XCUIElement` has no `scrollToVisible()` API.

## Correction

The paging workflow now locates the enclosing scroll view, performs bounded
user-like upward swipe gestures until the stable next-page control becomes
hittable, asserts hit testing, then clicks it. This tests the actual operator
interaction instead of relying on a nonexistent convenience API.

## Verification

- Repository Swift formatting gate.
- Canonical hosted Xcode checkpoint required after push; local host has no
  Xcode installation.

## Provenance

No external product reference influenced this test correction. It uses
TableRock-owned accessibility identifiers and standard XCUITest interaction.
