# Native bridge-error projection

Date: 2026-08-13

Checkpoint: P5a — remove the Presentation-to-Bridge error dependency

## Decision

Backend-specific rejection codes now cross the stable Feature boundary through
`WorkbenchCodedError` and `workbenchErrorCode(_:)`. The generated
`BridgeError` conforms in TableRockBridge, while presentation code depends only
on the Feature projection.

## Bounds and failure truth

- The external SQL-file conflict continues to recognize
  `sql-file-external-change` and require explicit overwrite confirmation.
- Uncoded errors continue to surface their localized description.
- Generated UniFFI error types remain confined to TableRockBridge and the app
  composition/development-support root.
- This checkpoint does not yet create the Presentation module or remove
  development fixtures from Release; those remain separate gates.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `scripts/verify-native-source-ownership.sh` passed.
- Focused Xcode tests passed: TableRockFeatureTests 37/0 and TableRockAppTests
  24/0.
- `scripts/verify-native-sql-files.sh` passed its structural and runtime gate.
- `git diff --check` passed.

## Clean-room provenance

This checkpoint changes dependency ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
