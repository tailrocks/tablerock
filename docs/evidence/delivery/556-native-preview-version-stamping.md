# 556 — Native preview version stamping

Date: 2026-07-21

## Decision

The direct native app builder accepts `TABLEROCK_APP_VERSION` and
`TABLEROCK_BUNDLE_VERSION`. The numeric public version remains
`CFBundleShortVersionString = 0.1.0`; rolling SemVer identity lives in the
TableRock-owned `TableRockPreviewVersion` key, and the positive commit counter
becomes `CFBundleVersion`. Static XML is updated through `plutil`, so
environment values are never interpolated into plist source.

Unset variables preserve the prior numeric values (`0.1.0`, `1`) and add the
matching preview identity. Empty app identity or a non-positive/non-numeric
bundle version fails before build work.

## Verification

```text
TABLEROCK_APP_VERSION=0.1.0-preview.1+abc1234 \
TABLEROCK_BUNDLE_VERSION=42 ./scripts/build-native-app.sh
plutil -p native/dist/TableRock.app/Contents/Info.plist
# CFBundleShortVersionString => 0.1.0
# CFBundleVersion => 42
# TableRockPreviewVersion => 0.1.0-preview.1+abc1234

./scripts/build-native-app.sh
plutil -extract CFBundleShortVersionString raw .../Info.plist # 0.1.0
plutil -extract CFBundleVersion raw .../Info.plist            # 1
plutil -extract TableRockPreviewVersion raw .../Info.plist    # 0.1.0
```

Both builds completed strict Swift 6 compilation and ad-hoc signing.

No external product influenced this packaging-only checkpoint.
