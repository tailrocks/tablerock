# Native control ownership

Date: 2026-07-22

## Hosted failure evidence

Exact commit `f262ab2`, native checkpoint `29877301426`, passed binding drift,
Swift tests, universal XCFramework construction, and 14 of 18 canonical Xcode
UI tests. Four failures proved evidence 618's presentation changes insufficient:

- an accessibility cell click still did not activate AppKit selection;
- CSV Stage still did not enter the accessibility hierarchy;
- CSV export still lacked a reliable hit point;
- SwiftUI exposed loaded-row status through AX value, not label.

## Structural correction

- Each reusable `NSTableCellView` now contains a borderless `NSButton` owning
  target/action, accessibility identity, label, and value. Selection no longer
  depends on a gesture recognizer competing with `NSTableView` event handling.
- Stage, Apply, and Discard remain stable sheet controls across every import
  state. Disabled state expresses unavailable authority instead of removing
  controls from the hierarchy.
- Result actions and filter/status controls use separate fixed toolbar rows, so
  narrow workbench content cannot overlap or compress action hit regions.
- Loaded-row status publishes dynamic content as accessibility value, matching
  macOS static-text projection and the existing TableRock status contract.

Backend behavior, safety gates, expected outcomes, and timeouts remain
unchanged.

## Local verification

```text
(cd native && swift build -c release)
# production package compiled

xcrun swiftc -parse native/Sources/TableRockApp/TableRockApp.swift
xcrun swiftc -parse native/Tests/TableRockAppUITests/TableRockAppUITests.swift
# production and canonical UI source parsed

git diff --check
# clean
```

Local XCTest remains unavailable under the installed Command Line Tools-only
developer directory. Exact-main hosted Xcode proof is required.

## Provenance

No external product source, test, identifier, product text, asset, screenshot,
layout measurement, color, or key binding influenced this correction. It
derives from TableRock's hosted XCUITest traces, native accessibility contract,
and plan 021.
