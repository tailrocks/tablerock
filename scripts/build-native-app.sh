#!/usr/bin/env bash
# Build a workable TableRock.app from the native SwiftUI target using Command
# Line Tools only (swiftc + the cargo release dylib + a manual .app bundle).
#
# No full Xcode and no Developer ID are required: SwiftUI + AppKit ship with the
# CLT macOS SDK. This produces an app that runs locally (ad-hoc signed). The
# notarized XCFramework distribution path (plan 019) remains operator-gated and
# is the release path; this script is the local-workable path (plan 020
# checkpoint 1).
#
# Usage: ./scripts/build-native-app.sh
# Output: native/dist/TableRock.app
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NATIVE="$REPO_ROOT/native"
DIST="$NATIVE/dist"
APP="$DIST/TableRock.app"
DYLIB_INSTALL_NAME="$REPO_ROOT/target/release/deps/libtablerock_ffi.dylib"

echo "==> Building Rust facade (release)"
cargo build -p tablerock-ffi --release

echo "==> Building SwiftUI app (swift build, debug — release optimizer needs the"
echo "    Xcode license via 'sudo xcodebuild -license', an operator step; debug runs fine)"
( cd "$NATIVE" && swift build --product TableRockApp )

echo "==> Bundling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Frameworks"
cp "$NATIVE/.build/debug/TableRockApp" "$APP/Contents/MacOS/TableRock"
cp "$REPO_ROOT/target/release/libtablerock_ffi.dylib" "$APP/Contents/Frameworks/"

# Rewrite the dylib reference so the bundle is self-contained (@rpath, not the
# absolute build path).
install_name_tool -id @rpath/libtablerock_ffi.dylib "$APP/Contents/Frameworks/libtablerock_ffi.dylib"
install_name_tool -change "$DYLIB_INSTALL_NAME" @rpath/libtablerock_ffi.dylib "$APP/Contents/MacOS/TableRock"
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
