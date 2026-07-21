#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE="$REPO_ROOT/native/Sources/TableRockApp/TableRockApp.swift"
DECODER="$REPO_ROOT/native/Sources/TableRockBridge/PageV1.swift"
TREE="$REPO_ROOT/native/Sources/TableRockFeature/StructuredValueTree.swift"
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
  'public struct PageV1Column' \
  'public struct PageV1Cell' \
  'originalByteCount' \
  'var kindLabel: String' \
  'columnMetadata: columnMetadata, cells: decodedCells'
do
  rg -q "$pattern" "$DECODER" || {
    echo "error: missing typed page decode contract: $pattern" >&2
    exit 1
  }
done

for pattern in \
  'maxInputBytes: Int = 64 \* 1024' \
  'maxNodes: Int = 1_024' \
  'maxDepth: Int = 64' \
  'object.keys.sorted()'
do
  rg -q "$pattern" "$TREE" || {
    echo "error: missing bounded JSON tree contract: $pattern" >&2
    exit 1
  }
done

for pattern in \
  'max\(tableView\.clickedColumn, 0\)' \
  'func tableViewSelectionDidChange' \
  'Text\("Value Inspector"\)' \
  'LabeledContent\("Database type"' \
  'LabeledContent\("Value kind"' \
  'GroupBox\("Text"\)' \
  'GroupBox\("Hex"\)' \
  'GroupBox\("JSON Tree"\)' \
  'StructuredValueTree.decode\(cell.bytes\)' \
  'cell\.isTruncated'
do
  rg -q "$pattern" "$SOURCE" || {
    echo "error: missing native inspector contract: $pattern" >&2
    exit 1
  }
done

pgrep -f "^$EXECUTABLE$" >/dev/null && {
  echo "error: TableRock already running" >&2
  exit 1
}
"$REPO_ROOT/scripts/build-native-app.sh" >/dev/null
audit_log="$(mktemp "$REPO_ROOT/target/native-value-inspector.XXXXXX")"
open -n -F --env TABLEROCK_FIXTURE_VALUE_INSPECTOR=1 \
  --stdout "$audit_log" --stderr "$audit_log" "$APP"
for _ in $(seq 1 50); do
  APP_PID="$(pgrep -n -f "^$EXECUTABLE$" || true)"
  [[ -n "$APP_PID" ]] && break
  sleep 0.1
done
for _ in $(seq 1 50); do
  rg -q '^VALUE_INSPECTOR_PROOF_' "$audit_log" && break
  sleep 0.1
done
if ! rg -q '^VALUE_INSPECTOR_PROOF_PASSED ' "$audit_log"; then
  cat "$audit_log" >&2
  echo "error: native value inspector runtime proof failed" >&2
  exit 1
fi

echo "native typed value inspector structural and runtime gate passed"
