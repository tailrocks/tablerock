# Evidence 654: native Vim mode

## Claim

TR-SCR-057 now has a native AppKit implementation over `NSTextView`, disabled
by default. An application-owned preference port isolates test namespaces and
persists the operator setting. Every query tab owns visible Insert/Normal state.
Normal mode consumes plain `h/j/k/l`, line delete, undo, and `i`; modified native
shortcuts pass through. Disabling the feature resets the editor to Insert.

Escape does not steal active marked-text composition. It enters Normal only
when `NSTextView` reports no marked text. Mode is shown as text with a stable
accessibility identifier/value. The feature explicitly documents its bounded
keymap rather than implying full Vim emulation.

## Verification

```text
mise exec -- ./scripts/build-native-app.sh --configuration Release
mise exec -- ./scripts/verify-native-vim-mode.sh
xcodebuild test -project native/App/TableRock.xcodeproj -scheme TableRock \
  -testPlan Checkpoint -destination 'platform=macOS'
```

Direct Swift 6 Release build passed. Shipped-app runtime audit passed Insert to
Normal, motion, line deletion, native undo, return to Insert, and IME Escape
preservation. Feature and AppKit unit tests plus scripted XCUITest are committed
to the canonical Checkpoint plan. Local Xcode execution is unavailable because
the active developer directory is CommandLineTools; hosted Xcode replay remains
the explicit status gap.

## Clean-room provenance

Current public TablePro editor/Vim workflow searches were performed before this
checkpoint; no accessible Vim-specific public documentation or screenshot was
found. TablePro influenced only the broad expectation that advanced editor
preferences remain discoverable. No TablePro source, tests, identifiers, text,
assets, colors, geometry, measurements, or key bindings were read or copied.
The bounded keymap derives from TableRock's existing TUI contract.
