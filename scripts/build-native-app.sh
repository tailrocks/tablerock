#!/usr/bin/env bash
# Build a workable TableRock.app using Command Line Tools ONLY — direct swiftc,
# no SwiftPM, no full Xcode, no Developer ID, no CLT license acceptance.
#
# Why direct swiftc: `swift build` (SwiftPM) and `cc` test-linking are gated on
# the Xcode license (`sudo xcodebuild -license`), an operator step. swiftc itself
# is not — so we compile the UniFFI bridge module + the SwiftUI app directly and
# link the cargo release dylib. This makes the workable app reproducible without
# any operator action. Notarized XCFramework distribution (plan 019) remains the
# separate operator-gated release path.
#
# Usage: ./scripts/build-native-app.sh
# Output: native/dist/TableRock.app
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NATIVE="$REPO_ROOT/native"
BUILD="$NATIVE/.build-direct"
DIST="$NATIVE/dist"
APP="$DIST/TableRock.app"
TARGET_arm64="arm64-apple-macos14"

echo "==> Building Rust facade (release)"
cargo build -p tablerock-ffi --release

echo "==> Building UniFFI bridge module (direct swiftc, no SwiftPM)"
rm -rf "$BUILD"
mkdir -p "$BUILD"
( cd "$NATIVE" \
  && swiftc -emit-module -module-name TableRockBridge \
       -swift-version 6 -strict-concurrency=complete -warnings-as-errors \
       -I Generated -Xcc -I -Xcc Generated -target "$TARGET_arm64" \
       -emit-module-path "$BUILD/TableRockBridge.swiftmodule" \
       Sources/TableRockBridge/tablerock_ffi.swift Sources/TableRockBridge/PageV1.swift \
  && swiftc -c -module-name TableRockBridge \
       -swift-version 6 -strict-concurrency=complete -warnings-as-errors \
       -I Generated -Xcc -I -Xcc Generated -target "$TARGET_arm64" \
       Sources/TableRockBridge/tablerock_ffi.swift Sources/TableRockBridge/PageV1.swift \
  && mv tablerock_ffi.o PageV1.o "$BUILD/" )

echo "==> Building SwiftUI app (direct swiftc)"
( cd "$NATIVE" \
  && swiftc -parse-as-library \
       -swift-version 6 -strict-concurrency=complete -warnings-as-errors \
       -I "$BUILD" -I Generated -Xcc -I -Xcc Generated -target "$TARGET_arm64" \
       Sources/TableRockApp/*.swift \
       "$BUILD/tablerock_ffi.o" "$BUILD/PageV1.o" \
       -L "$REPO_ROOT/target/release" -ltablerock_ffi \
       -framework SwiftUI -framework AppKit \
       -o "$BUILD/TableRockApp" )

echo "==> Bundling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Frameworks"
cp "$BUILD/TableRockApp" "$APP/Contents/MacOS/TableRock"
cp "$REPO_ROOT/target/release/libtablerock_ffi.dylib" "$APP/Contents/Frameworks/"

# Rewrite the dylib reference so the bundle is self-contained (@rpath, not the
# absolute build path), then point the executable at the embedded Frameworks dir.
install_name_tool -id @rpath/libtablerock_ffi.dylib "$APP/Contents/Frameworks/libtablerock_ffi.dylib"
install_name_tool -change "$REPO_ROOT/target/release/deps/libtablerock_ffi.dylib" @rpath/libtablerock_ffi.dylib "$APP/Contents/MacOS/TableRock"
install_name_tool -add_rpath @executable_path/../Frameworks "$APP/Contents/MacOS/TableRock"

cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>TableRock</string>
  <key>CFBundleIdentifier</key><string>app.tablerock.TableRock</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleExecutable</key><string>TableRock</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>14.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# Ad-hoc sign (install_name_tool invalidated the build signature). Local-run only;
# notarization is the operator-gated release path.
codesign -s - --force --deep "$APP"

echo "==> Built $APP"
echo "    open it: open $APP"
