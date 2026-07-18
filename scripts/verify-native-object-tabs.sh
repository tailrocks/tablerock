#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE="$REPO_ROOT/native/Sources/TableRockApp/TableRockApp.swift"
BRIDGE="$REPO_ROOT/crates/tablerock-ffi/src/bridge.rs"
APP="$REPO_ROOT/native/dist/TableRock.app"
EXECUTABLE="$APP/Contents/MacOS/TableRock"
APP_PID=""

cleanup() {
  if [[ -n "$APP_PID" ]] && kill -0 "$APP_PID" 2>/dev/null; then
    kill "$APP_PID" 2>/dev/null || true
    wait "$APP_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

for pattern in \
  'final class NativeObjectTab' \
  'func openCatalogObject\(nodeKey: String\) async' \
  'activeObjectTab\?\.pinned = true' \
  'func pinObjectTab' \
  'func closeObjectTab' \
  'ObjectWorkbenchView\(\)' \
  'doubleAction = #selector\(Coordinator.openSelectedObject\)'
do
  rg -q "$pattern" "$SOURCE" || {
    echo "error: missing native object-tab contract: $pattern" >&2
    exit 1
  }
done
for pattern in \
  'pub fn submit_catalog_browse' \
  'BrowsePlan \{' \
  'catalog node is not a browsable table-like object' \
  '"browse_object"'
do
  rg -q "$pattern" "$BRIDGE" || {
    echo "error: missing Rust catalog-browse contract: $pattern" >&2
    exit 1
  }
done

pgrep -f "^$EXECUTABLE$" >/dev/null && {
  echo "error: TableRock already running" >&2
  exit 1
}
"$REPO_ROOT/scripts/build-native-app.sh" >/dev/null
audit_log="$(mktemp "$REPO_ROOT/target/native-object-tabs.XXXXXX")"
open -n -F --env TABLEROCK_FIXTURE_OBJECT_TABS=1 \
  --stdout "$audit_log" --stderr "$audit_log" "$APP"
for _ in $(seq 1 50); do
  APP_PID="$(pgrep -n -f "^$EXECUTABLE$" || true)"
  [[ -n "$APP_PID" ]] && break
  sleep 0.1
done
for _ in $(seq 1 50); do
  rg -q '^OBJECT_TABS_PROOF_' "$audit_log" && break
  sleep 0.1
done
if ! rg -q '^OBJECT_TABS_PROOF_PASSED ' "$audit_log"; then
  cat "$audit_log" >&2
  echo "error: native object-tab runtime proof failed" >&2
  exit 1
fi

echo "native object-tab structural and runtime gate passed"
