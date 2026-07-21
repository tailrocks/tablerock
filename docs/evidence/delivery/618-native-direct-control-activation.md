# Native direct control activation

Date: 2026-07-22

## Failure evidence

Exact commit `6be9688`, native checkpoint `29875658677`, passed binding drift,
Swift tests, and universal XCFramework construction. Canonical Xcode then
passed 14 of 18 UI tests and exposed four concrete presentation defects:

- a result cell received a click but owned no installed gesture recognizer;
- CSV Stage remained absent because authority actions shared a clipped title
  row;
- the SwiftUI export menu existed but had no reliable AX hit point;
- loaded-row status existed with an empty AX label.

## Correction

- Every new AppKit result cell installs its own click recognizer and routes it
  through the same row/column selection method as table and AX actions.
- CSV sheet Close remains in the title row; Stage/Apply/Discard use a separate
  fixed action row above scrolling content.
- CSV export is a direct identified button. Other formats remain under an
  independently identified menu.
- Loaded-row status explicitly publishes its full visible text as AX label.

No test assertion, timeout, backend behavior, or safety gate was weakened.

## Verification

```text
(cd native && swift build -c release)
# production package compiled

xcrun swiftc -parse native/Tests/TableRockAppUITests/TableRockAppUITests.swift
# parsed

git diff --check
# clean
```

Exact-main hosted Xcode proof remains required.

## Provenance

No external product source, test, identifier, product text, asset, screenshot,
layout measurement, color, or key binding influenced this correction. It
derives from TableRock's XCUITest log, accessibility contract, and plan 021.
