#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FFI="$REPO_ROOT/crates/tablerock-ffi/src/bridge.rs"
SOURCE="$REPO_ROOT/native/Sources/TableRockApp/TableRockApp.swift"
BINDING="$REPO_ROOT/native/Sources/TableRockBridge/tablerock_ffi.swift"

for pattern in \
  'pub fn start_table_operation' \
  'pub fn table_operation_status' \
  'pub fn dismiss_table_operation' \
  'cancellable: false'
do
  rg -q "$pattern" "$FFI" || { echo "error: missing Rust maintenance lifecycle: $pattern" >&2; exit 1; }
done

for pattern in \
  'func startTableOperation' \
  'func tableOperationStatus' \
  'while true' \
  'table-operation.progress' \
  'table-operation.cancel-unavailable'
do
  rg -q "$pattern" "$SOURCE" || { echo "error: missing native maintenance state: $pattern" >&2; exit 1; }
done

rg -q 'open func startTableOperation' "$BINDING" || {
  echo "error: generated Swift binding lacks maintenance start" >&2
  exit 1
}
rg -q 'open func tableOperationStatus' "$BINDING" || {
  echo "error: generated Swift binding lacks maintenance status" >&2
  exit 1
}

mise exec -- bash "$REPO_ROOT/scripts/build-native-app.sh" >/dev/null
echo "native PostgreSQL maintenance lifecycle gate passed"
