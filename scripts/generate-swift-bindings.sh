#!/usr/bin/env bash
# Generate UniFFI Swift bindings from the built tablerock_ffi library.
# Deterministic: always writes the same outputs for a given library binary.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${OUT_DIR:-$ROOT/native/Generated}"
PROFILE="${PROFILE:-release}"
LIB="$ROOT/target/$PROFILE/libtablerock_ffi.dylib"

cd "$ROOT"

echo "==> building tablerock-ffi ($PROFILE)"
cargo build -p tablerock-ffi --"$PROFILE"

if [[ ! -f "$LIB" ]]; then
  echo "error: expected library at $LIB" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"

echo "==> generating Swift bindings into $OUT_DIR"
# Prefer installed uniffi CLI; fall back to cargo-run helper binary if present.
if command -v uniffi-bindgen >/dev/null 2>&1; then
  uniffi-bindgen generate --library "$LIB" --language swift --out-dir "$OUT_DIR"
elif [[ -x "$ROOT/target/$PROFILE/uniffi-bindgen-tablerock" ]]; then
  "$ROOT/target/$PROFILE/uniffi-bindgen-tablerock" generate --library "$LIB" --language swift --out-dir "$OUT_DIR"
else
  cargo run -p tablerock-ffi --features bindgen-cli --bin uniffi-bindgen -- \
    generate --library "$LIB" --language swift --out-dir "$OUT_DIR"
fi

# Keep Sources/TableRockBridge in sync for the Swift package (committed artifact).
SOURCES_SWIFT="$ROOT/native/Sources/TableRockBridge"
mkdir -p "$SOURCES_SWIFT"
if [[ -f "$OUT_DIR/tablerock_ffi.swift" ]]; then
  cp "$OUT_DIR/tablerock_ffi.swift" "$SOURCES_SWIFT/tablerock_ffi.swift"
fi
if [[ -f "$OUT_DIR/tablerock_ffiFFI.modulemap" && ! -f "$OUT_DIR/module.modulemap" ]]; then
  # SPM systemLibrary expects module.modulemap at the system-lib root.
  cat >"$OUT_DIR/module.modulemap" <<'EOF'
module tablerock_ffiFFI {
    header "tablerock_ffiFFI.h"
    export *
}
EOF
fi

echo "==> generated:"
ls -la "$OUT_DIR"
ls -la "$SOURCES_SWIFT"
