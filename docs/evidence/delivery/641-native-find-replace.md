# Evidence 641: native Find and Replace

Date: 2026-07-22

## Outcome

`TR-SCR-049` now has a native editor command and shipped sheet with:

- case-insensitive literal, case-sensitive literal, Unicode whole-word, and
  regular-expression modes;
- explicit whole-document or frozen current-selection scope;
- previous/next navigation with wrap, plus replace and replace-all;
- synchronized `NSTextView` selection without replacing active IME marked text;
- invalid-pattern, empty-pattern, empty-selection, no-match, and outcome states;
- a 10,000-match replace-all bound;
- finite zero-width regular-expression traversal and replacement.

Model coverage exercises whole-word case folding, frozen selection scope, and
a zero-width Unicode look-ahead replacement. XCUITest opens the production
command sheet, enters values, replaces through the shipped editor, observes the
status, and dismisses.

## Verification

```text
xcrun swift-format format --in-place <changed Swift files>
passed

mise exec -- ./scripts/build-native-app.sh --configuration Release
Built native/dist/TableRock.app

swift test --package-path native -c release
local STOP: Command Line Tools Swift lacks XCTest; full Xcode is not installed
hosted Native Checkpoint: pending on completion commit
```

`git diff --check` passes. Hosted XCTest/XCUITest result must replace the
pending line before this evidence can support final closure.

## Clean-room provenance

TablePro's current public site was checked for broad database-editor workflow
expectations; it exposed no specific Find and Replace contract during this
checkpoint. No TablePro source, tests, strings, assets, screenshots, layout
measurements, colors, or key bindings were read or copied. TableRock behavior
and presentation derive independently from repository requirements, AppKit
text-system behavior, Foundation regular expressions, and direct tests.
