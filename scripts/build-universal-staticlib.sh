#!/usr/bin/env bash
# Build universal (lipo) macOS staticlib for tablerock-ffi without requiring
# full Xcode / xcodebuild -create-xcframework.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${OUT_DIR:-$ROOT/target/universal}"
cd "$ROOT"

echo "==> rustup targets"
rustup target add aarch64-apple-darwin x86_64-apple-darwin

echo "==> release staticlibs"
cargo build -p tablerock-ffi --release --target aarch64-apple-darwin
cargo build -p tablerock-ffi --release --target x86_64-apple-darwin

ARM="$ROOT/target/aarch64-apple-darwin/release/libtablerock_ffi.a"
X86="$ROOT/target/x86_64-apple-darwin/release/libtablerock_ffi.a"
mkdir -p "$OUT_DIR"
UNIVERSAL="$OUT_DIR/libtablerock_ffi.a"

echo "==> lipo universal staticlib"
lipo -create -output "$UNIVERSAL" "$ARM" "$X86"
lipo -info "$UNIVERSAL"
ls -la "$UNIVERSAL"

# Also regenerate Swift bindings from host dylib.
cargo build -p tablerock-ffi --release
PROFILE=release bash "$ROOT/scripts/generate-swift-bindings.sh"

echo "==> universal staticlib ready: $UNIVERSAL"
echo "    XCFramework still requires full Xcode (scripts/build-xcframework.sh)."
