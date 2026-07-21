# Native UI hit-testing correction

Date: 2026-07-21

## Hosted evidence

Native checkpoint run 29849426804 improved the canonical suite to ten of
fourteen UI tests passing. The new app-status and connection-status contracts
proved correct: both IME preservation and temporary connection passed.

The four residual failures identified concrete macOS interaction boundaries:

- XCTest predicates do not expose `isHittable` as a query key path.
- `NSTableCellView` is not itself an exposed accessibility element.
- the sheet toolbar's confirmation button had no default keyboard action and
  its synthesized click missed the toolbar hit target.
- the paging button needed scrolling into the visible AppKit viewport before
  activation.

## Correction

- The dirty-tab test enumerates matching menu items and selects the element
  whose XCTest property reports it hittable.
- Result-grid identifiers are attached to the accessible `NSTextField` as well
  as the cell container.
- Profile Save is a standard default action, exercised through Return after
  editing the real form.
- Paging scrolls the shipped button visible before clicking and still requires
  terminal disappearance plus the exact 501-row summary.

## Verification

- `git diff --check`: passed.
- Run 29849426804: all non-UI Xcode tests passed; 10/14 UI tests passed.
- Corrected hosted rerun pending after push.

## Provenance

No external product reference influenced this correction. Evidence comes from
TableRock's hosted XCTest activity log and its AppKit/SwiftUI accessibility
implementation.
