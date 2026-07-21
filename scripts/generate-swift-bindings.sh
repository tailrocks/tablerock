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
  cargo run -p tablerock-ffi --features bindgen-cli --bin uniffi-bindgen-tablerock -- \
    generate --library "$LIB" --language swift --out-dir "$OUT_DIR"
fi

# UniFFI templates contain trailing spaces. Normalize committed artifacts so
# generation is stable under the repository whitespace gate.
for generated in "$OUT_DIR/tablerock_ffi.swift" "$OUT_DIR/tablerock_ffiFFI.h"; do
  [[ ! -f "$generated" ]] || perl -pi -e 's/[ \t]+$//' "$generated"
done

# Keep Sources/TableRockBridge in sync for the Swift package (committed artifact).
SOURCES_SWIFT="$ROOT/native/Sources/TableRockBridge"
mkdir -p "$SOURCES_SWIFT"
if [[ -f "$OUT_DIR/tablerock_ffi.swift" ]]; then
  cp "$OUT_DIR/tablerock_ffi.swift" "$SOURCES_SWIFT/tablerock_ffi.swift"
fi
# UniFFI does not encode native dependencies of the Rust static library. Keep
# those dependencies on the C module so every Swift consumer links them.
for module_map in "$OUT_DIR/tablerock_ffiFFI.modulemap" "$OUT_DIR/module.modulemap"; do
  cat >"$module_map" <<'EOF'
module tablerock_ffiFFI {
    header "tablerock_ffiFFI.h"
    export *
    use "Darwin"
    use "_Builtin_stdbool"
    use "_Builtin_stdint"
    link framework "SystemConfiguration"
    link framework "CoreFoundation"
    link framework "Security"
    link "iconv"
}
EOF
done

echo "==> generated:"
ls -la "$OUT_DIR"
ls -la "$SOURCES_SWIFT"
