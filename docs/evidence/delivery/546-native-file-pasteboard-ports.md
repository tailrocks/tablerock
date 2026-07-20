# 546 — Native file-panel and pasteboard ports

Date: 2026-07-21

## Decision

Plan 021 checkpoint 12 now routes all production model file selection and
pasteboard publication through application-owned ports:

- `AppFilePanelPort` accepts immutable open/save request facts and returns a
  selected URL or cancellation;
- `AppPasteboardPort` accepts typed string representations without importing
  AppKit into `TableRockFeature`;
- production-only adapters own `NSOpenPanel`, `NSSavePanel`, and
  `NSPasteboard.general`;
- unavailable defaults cancel panels and reject pasteboard writes, so an
  unconfigured test cannot touch operator state.

Loaded-result export, CSV import, SQL open/save, multi-representation result
copy, and structure-DDL copy now use the ports. Security-scoped URL lifetime
remains in the presentation adapter around Rust-owned file operations.

## Failure truth

Panel cancellation produces no effect. Pasteboard rejection becomes the
existing visible copy error. The model never falls back to a global clipboard
or path. Tests inject recording ports and assert exact request/payload facts.

## Verification

```text
./scripts/build-native-app.sh
./scripts/verify-native-accessibility.sh
```

Strict Swift 6 Release build and launched-app runtime accessibility gate pass.
The new feature tests are compiled/executed by the future full-Xcode checkpoint
gate; local Command Line Tools still lack Swift's `Testing` module.

## Remaining checkpoint 12 work

- application-owned `WorkbenchBackend` DTO boundary and live adapter;
- scripted backend scenarios and model-deallocation coverage;
- isolated Keychain capability namespace.

TablePro establishes the broad native file/clipboard workflow need only.
Copied source, tests, identifiers, assets, product text, screenshots, layout
measurements, colors, or key bindings: none.
