#!/usr/bin/env bash
# Build a universal XCFramework for tablerock-ffi (macOS aarch64 + x86_64).
# Does not sign or notarize — those steps need operator Developer ID credentials.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${OUT_DIR:-$ROOT/target/xcframework}"
FRAMEWORK_NAME="TableRockFFI"
XCFRAMEWORK="$OUT_DIR/$FRAMEWORK_NAME.xcframework"

cd "$ROOT"

echo "==> building staticlibs for apple-darwin targets"
rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
cargo build -p tablerock-ffi --release --target aarch64-apple-darwin
cargo build -p tablerock-ffi --release --target x86_64-apple-darwin

ARM_LIB="$ROOT/target/aarch64-apple-darwin/release/libtablerock_ffi.a"
X86_LIB="$ROOT/target/x86_64-apple-darwin/release/libtablerock_ffi.a"

for lib in "$ARM_LIB" "$X86_LIB"; do
  if [[ ! -f "$lib" ]]; then
    echo "error: missing $lib" >&2
    exit 1
  fi
done

echo "==> generating Swift bindings (from host release dylib)"
cargo build -p tablerock-ffi --release
PROFILE=release OUT_DIR="$ROOT/native/Generated" \
  bash "$ROOT/scripts/generate-swift-bindings.sh"

HEADER="$ROOT/native/Generated/tablerock_ffiFFI.h"
MODULEMAP="$ROOT/native/Generated/tablerock_ffiFFI.modulemap"
if [[ ! -f "$HEADER" ]]; then
  # UniFFI 0.32 may name headers differently; pick the first .h
  HEADER="$(find "$ROOT/native/Generated" -name '*.h' | head -n1)"
fi
if [[ ! -f "$HEADER" ]]; then
  echo "error: no generated header under native/Generated" >&2
  exit 1
fi

rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR/macos-arm64" "$OUT_DIR/macos-x86_64"

# Thin static frameworks (headers + module map + .a) for each arch.
package_arch() {
  local arch_dir="$1"
  local lib="$2"
  local framework="$arch_dir/$FRAMEWORK_NAME.framework"
  mkdir -p "$framework/Headers" "$framework/Modules"
  cp "$lib" "$framework/$FRAMEWORK_NAME"
  cp "$HEADER" "$framework/Headers/"
  if [[ -f "$MODULEMAP" ]]; then
    cp "$MODULEMAP" "$framework/Modules/module.modulemap"
  else
    cat >"$framework/Modules/module.modulemap" <<EOF
framework module $FRAMEWORK_NAME {
  umbrella header "$(basename "$HEADER")"
  export *
  module * { export * }
}
EOF
  fi
}

package_arch "$OUT_DIR/macos-arm64" "$ARM_LIB"
package_arch "$OUT_DIR/macos-x86_64" "$X86_LIB"

echo "==> creating XCFramework"
xcodebuild -create-xcframework \
  -framework "$OUT_DIR/macos-arm64/$FRAMEWORK_NAME.framework" \
  -framework "$OUT_DIR/macos-x86_64/$FRAMEWORK_NAME.framework" \
  -output "$XCFRAMEWORK"

echo "==> XCFramework ready: $XCFRAMEWORK"
ls -la "$XCFRAMEWORK"
