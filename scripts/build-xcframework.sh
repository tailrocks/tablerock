#!/usr/bin/env bash
# Build a universal XCFramework for tablerock-ffi (macOS aarch64 + x86_64).
# Does not sign or notarize — those steps need operator Developer ID credentials.
set -euo pipefail

export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-26.0}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${OUT_DIR:-$ROOT/target/xcframework}"
# UniFFI-generated Swift imports this exact low-level C module. Framework,
# binary, and module-map identities must match so canImport() succeeds in Xcode.
FRAMEWORK_NAME="tablerock_ffiFFI"
XCFRAMEWORK="$OUT_DIR/$FRAMEWORK_NAME.xcframework"

cd "$ROOT"

if [[ "${SKIP_BINDINGS:-0}" == "1" ]]; then
  echo "==> using previously verified Swift bindings"
else
  echo "==> generating Swift bindings (from host release dylib)"
  cargo build -p tablerock-ffi --release --locked
  PROFILE=release OUT_DIR="$ROOT/native/Generated" \
    bash "$ROOT/scripts/generate-swift-bindings.sh"
fi

echo "==> building staticlibs for apple-darwin targets"
rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null

HOST_LIB="$ROOT/target/release/libtablerock_ffi.a"
ARM_LIB="$ROOT/target/aarch64-apple-darwin/release/libtablerock_ffi.a"
X86_LIB="$ROOT/target/x86_64-apple-darwin/release/libtablerock_ffi.a"

if [[ -f "$HOST_LIB" && "$(lipo -archs "$HOST_LIB")" == "arm64" ]]; then
  echo "==> reusing verified host arm64 staticlib"
  ARM_LIB="$HOST_LIB"
else
  cargo build -p tablerock-ffi --release --locked --target aarch64-apple-darwin
fi

if [[ -f "$HOST_LIB" && "$(lipo -archs "$HOST_LIB")" == "x86_64" ]]; then
  echo "==> reusing verified host x86_64 staticlib"
  X86_LIB="$HOST_LIB"
else
  cargo build -p tablerock-ffi --release --locked --target x86_64-apple-darwin
fi

for lib in "$ARM_LIB" "$X86_LIB"; do
  if [[ ! -f "$lib" ]]; then
    echo "error: missing $lib" >&2
    exit 1
  fi
done

HEADER="$ROOT/native/Generated/tablerock_ffiFFI.h"
if [[ ! -f "$HEADER" ]]; then
  # UniFFI 0.32 may name headers differently; pick the first .h
  HEADER="$(find "$ROOT/native/Generated" -name '*.h' | head -n1)"
fi
if [[ ! -f "$HEADER" ]]; then
  echo "error: no generated header under native/Generated" >&2
  exit 1
fi

rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR/macos-universal"

# Xcode rejects separate arm64 and x86_64 framework inputs because they are
# equivalent macOS library definitions. Package one universal macOS framework;
# the XCFramework may later gain non-macOS platform variants as separate slices.
FRAMEWORK="$OUT_DIR/macos-universal/$FRAMEWORK_NAME.framework"
mkdir -p "$FRAMEWORK/Headers" "$FRAMEWORK/Modules"
lipo -create "$ARM_LIB" "$X86_LIB" -output "$FRAMEWORK/$FRAMEWORK_NAME"
cp "$HEADER" "$FRAMEWORK/Headers/"
cat >"$FRAMEWORK/Modules/module.modulemap" <<EOF
framework module $FRAMEWORK_NAME {
  umbrella header "$(basename "$HEADER")"
  export *
  module * { export * }
}
EOF

echo "==> creating XCFramework"
xcodebuild -create-xcframework \
  -framework "$FRAMEWORK" \
  -output "$XCFRAMEWORK"

echo "==> XCFramework ready: $XCFRAMEWORK"
ls -la "$XCFRAMEWORK"
