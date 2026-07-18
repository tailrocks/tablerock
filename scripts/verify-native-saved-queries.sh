#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE="$REPO_ROOT/native/Sources/TableRockApp/TableRockApp.swift"
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
  'Label\("Saved Queries", systemImage: "bookmark"\)' \
  'Label\("Save Query", systemImage: "bookmark.badge.plus"\)' \
  'searchable\(text: \$model.savedQuerySearch' \
  'Text\("All engines"\).tag\(""\)' \
  'Restore into the editor without running it' \
  'Remove saved query\?' \
  'Query history is unchanged.'
do
  rg -q "$pattern" "$SOURCE" || {
    echo "error: missing native saved-query contract: $pattern" >&2
    exit 1
  }
done

pgrep -f "^$EXECUTABLE$" >/dev/null && {
  echo "error: TableRock already running" >&2
  exit 1
}
"$REPO_ROOT/scripts/build-native-app.sh" >/dev/null
audit_log="$(mktemp "$REPO_ROOT/target/native-saved-queries.XXXXXX")"
open -n -F --env TABLEROCK_FIXTURE_SAVED_QUERIES=1 \
  --stdout "$audit_log" --stderr "$audit_log" "$APP"
for _ in $(seq 1 50); do
  APP_PID="$(pgrep -n -f "^$EXECUTABLE$" || true)"
  [[ -n "$APP_PID" ]] && break
  sleep 0.1
done
for _ in $(seq 1 50); do
  rg -q '^SAVED_QUERIES_PROOF_' "$audit_log" && break
  sleep 0.1
done
if ! rg -q '^SAVED_QUERIES_PROOF_PASSED ' "$audit_log"; then
  cat "$audit_log" >&2
  echo "error: native saved-query runtime proof failed" >&2
  exit 1
fi

echo "native saved-query structural and runtime gate passed"
