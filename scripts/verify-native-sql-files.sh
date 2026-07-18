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
  'NSOpenPanel\(\)' \
  'NSSavePanel\(\)' \
  'startAccessingSecurityScopedResource\(\)' \
  'stopAccessingSecurityScopedResource\(\)' \
  'Discard unsaved editor changes\?' \
  'SQL file changed outside TableRock' \
  'Reload External Changes' \
  'Overwrite External Changes'
do
  rg -q "$pattern" "$SOURCE" || {
    echo "error: missing native SQL-file contract: $pattern" >&2
    exit 1
  }
done

pgrep -f "^$EXECUTABLE$" >/dev/null && {
  echo "error: TableRock already running" >&2
  exit 1
}
"$REPO_ROOT/scripts/build-native-app.sh" >/dev/null
audit_log="$(mktemp "$REPO_ROOT/target/native-sql-files.XXXXXX")"
open -n -F --env TABLEROCK_FIXTURE_SQL_FILES=1 \
  --stdout "$audit_log" --stderr "$audit_log" "$APP"
for _ in $(seq 1 50); do
  APP_PID="$(pgrep -n -f "^$EXECUTABLE$" || true)"
  [[ -n "$APP_PID" ]] && break
  sleep 0.1
done
for _ in $(seq 1 50); do
  rg -q '^SQL_FILES_PROOF_' "$audit_log" && break
  sleep 0.1
done
if ! rg -q '^SQL_FILES_PROOF_PASSED ' "$audit_log"; then
  cat "$audit_log" >&2
  echo "error: native SQL-file runtime proof failed" >&2
  exit 1
fi

echo "native SQL-file structural and runtime gate passed"
