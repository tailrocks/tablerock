# Native UI semantic observation correction

Date: 2026-07-21

## Hosted evidence

Native checkpoint run 29847434527 proved the launch-isolation correction:
eight of fourteen UI tests passed, including workbench launch, multi-window,
settings, cancellation, structured inspection, catalog refresh, accessibility,
and the large grid. The prior systemic missing-window failures disappeared.

Its six residual failures exposed independent observation errors:

- AppKit result cells do not promise XCTest's `.cell` role.
- SwiftUI exposes duplicate hidden and visible `Close` menu items.
- connection and IME lifecycle state are not query-result state.
- profile creation is durably proved by the new profile control, not a
  transient outcome label behind sheet dismissal.
- paging completion is authoritatively signalled by exhaustion of the next-page
  control before the updated summary is read.

## Correction

- App lifecycle and connected-session labels now have stable semantic
  accessibility identifiers and values.
- Tests query the AppKit cell by identifier independent of role.
- Dirty-tab close selects the visible `Close` menu item.
- Profile creation awaits the persisted profile control.
- Result paging awaits disappearance of `results.next-page`, then verifies the
  exact 501-row summary.

These assertions continue driving shipped controls and production model paths;
no test-only bypass or alternate UI was added.

## Verification

- `git diff --check`: passed.
- Run 29847434527: all non-UI Xcode tests passed; 8/14 UI tests passed and each
  residual failure was matched to the corrected semantic boundary above.
- Corrected hosted rerun pending after push.

## Provenance

No external product reference influenced this correction. Evidence comes from
TableRock's hosted Xcode activity log, AppKit/SwiftUI accessibility hierarchy,
and repository-defined state contracts.
