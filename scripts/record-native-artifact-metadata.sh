#!/usr/bin/env bash
# Record deterministic structural metadata for a canonical native archive.
set -euo pipefail

if [[ "$#" -ne 3 ]]; then
  echo "usage: $0 ARCHIVE XCFRAMEWORK OUTPUT_DIRECTORY" >&2
  exit 2
fi

ARCHIVE="$1"
XCFRAMEWORK="$2"
OUT="$3"
APP="$ARCHIVE/Products/Applications/TableRock.app"
EXECUTABLE="$APP/Contents/MacOS/TableRock"
BRIDGE="$XCFRAMEWORK/macos-arm64_x86_64/tablerock_ffiFFI.framework/tablerock_ffiFFI"

test -d "$APP"
test -f "$EXECUTABLE"
test -f "$BRIDGE"
mkdir -p "$OUT"

plutil -convert json -o "$OUT/app-info.json" "$APP/Contents/Info.plist"
lipo -archs "$EXECUTABLE" > "$OUT/app-architectures.txt"
lipo -archs "$BRIDGE" > "$OUT/bridge-architectures.txt"
otool -L "$EXECUTABLE" > "$OUT/app-linkage.txt"
codesign --display --verbose=4 "$APP" 2> "$OUT/codesign.txt"
shasum -a 256 "$EXECUTABLE" "$BRIDGE" > "$OUT/sha256.txt"

if rg -q '/target/|libtablerock_ffi[.]dylib' "$OUT/app-linkage.txt"; then
  echo "error: canonical app retains development bridge linkage" >&2
  exit 1
fi
