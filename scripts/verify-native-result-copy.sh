#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE="$REPO_ROOT/native/Sources/TableRockApp/TableRockApp.swift"
CORE="$REPO_ROOT/crates/tablerock-core/src/copy_projection.rs"
FFI="$REPO_ROOT/crates/tablerock-ffi/src/bridge.rs"
TUI="$REPO_ROOT/crates/tablerock-tui/src/model/copy_format.rs"
APP="$REPO_ROOT/native/dist/TableRock.app"
EXECUTABLE="$APP/Contents/MacOS/TableRock"
CONTAINER="tablerock-native-copy-pg"
APP_PID=""
EXPORT_DIR=""
EXPORT_PATH=""
STREAM_EXPORT_PATH=""

cleanup() {
  if [[ -n "$APP_PID" ]] && kill -0 "$APP_PID" 2>/dev/null; then
    kill "$APP_PID" 2>/dev/null || true
    wait "$APP_PID" 2>/dev/null || true
  fi
  docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
  if [[ -n "$EXPORT_PATH" ]]; then rm -f "$EXPORT_PATH"; fi
  if [[ -n "$STREAM_EXPORT_PATH" ]]; then rm -f "$STREAM_EXPORT_PATH"; fi
  if [[ -n "$EXPORT_DIR" ]]; then rmdir "$EXPORT_DIR" 2>/dev/null || true; fi
}
trap cleanup EXIT

for pattern in \
  'pub fn format_copy_table' \
  'const MAX_COPY_BYTES: usize = 16 \* 1024 \* 1024' \
  'CopyProjectionError::MissingStableIdentity' \
  'CopyCell::Truncated'
do
  rg -q "$pattern" "$CORE" || { echo "error: missing shared copy contract: $pattern" >&2; exit 1; }
done
rg -q 'format_copy_table\(&table, format\)' "$TUI" || {
  echo "error: TUI does not use shared copy formatter" >&2; exit 1;
}
rg -q 'pub fn format_result_copy' "$FFI" || {
  echo "error: FFI result-copy handle missing" >&2; exit 1;
}
rg -q 'pub fn export_loaded_result' "$FFI" || {
  echo "error: FFI atomic result export missing" >&2; exit 1;
}
rg -q 'pub fn start_stream_export' "$FFI" || {
  echo "error: FFI streaming result export missing" >&2; exit 1;
}
for pattern in \
  'NSPasteboardItem\(\)' \
  'forType: \.tabularText' \
  'forType: \.init\("public.json"\)' \
  'forType: \.init\("net.daringfireball.markdown"\)' \
  'Button\("SQL INSERT"\)' \
  'ResultExportMenu\(\)' \
  'startAccessingSecurityScopedResource\(\)' \
  'selectedWorkbenchKind == "object"' \
  'if !model.queryWorkbenchSelected'
do
  rg -q "$pattern" "$SOURCE" || { echo "error: missing native copy contract: $pattern" >&2; exit 1; }
done

docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
docker run -d --name "$CONTAINER" \
  -e POSTGRES_PASSWORD=secret -e POSTGRES_USER=u -e POSTGRES_DB=db \
  -p 5433:5432 postgres:18.4-alpine >/dev/null
for i in $(seq 1 30); do
  docker exec "$CONTAINER" pg_isready -U u -d db >/dev/null 2>&1 \
    && docker exec "$CONTAINER" psql -U u -d db -c 'SELECT 1' >/dev/null 2>&1 \
    && break
  sleep 1
  [[ "$i" -eq 30 ]] && { echo "error: PostgreSQL not ready" >&2; exit 1; }
done

pgrep -f "^$EXECUTABLE$" >/dev/null && {
  echo "error: TableRock already running" >&2
  exit 1
}
"$REPO_ROOT/scripts/build-native-app.sh" >/dev/null
audit_log="$(mktemp "$REPO_ROOT/target/native-result-copy.XXXXXX")"
EXPORT_DIR="$(mktemp -d "$REPO_ROOT/target/native-result-export.XXXXXX")"
EXPORT_PATH="$EXPORT_DIR/result.json"
STREAM_EXPORT_PATH="$EXPORT_DIR/full.csv"
open -n -F --env TABLEROCK_FIXTURE_RESULT_COPY=1 \
  --env TABLEROCK_FIXTURE_RESULT_EXPORT_PATH="$EXPORT_PATH" \
  --env TABLEROCK_FIXTURE_STREAM_EXPORT_PATH="$STREAM_EXPORT_PATH" \
  --stdout "$audit_log" --stderr "$audit_log" "$APP"
for _ in $(seq 1 50); do
  APP_PID="$(pgrep -n -f "^$EXECUTABLE$" || true)"
  [[ -n "$APP_PID" ]] && break
  sleep 0.1
done
for _ in $(seq 1 80); do
  rg -q '^RESULT_COPY_PROOF_' "$audit_log" && break
  sleep 0.1
done
if ! rg -q '^RESULT_COPY_PROOF_PASSED ' "$audit_log"; then
  cat "$audit_log" >&2
  echo "error: native result-copy runtime proof failed" >&2
  exit 1
fi
if [[ ! -f "$STREAM_EXPORT_PATH" ]] || ! rg -q '^1200$' "$STREAM_EXPORT_PATH"; then
  cat "$audit_log" >&2
  echo "error: native full-result streaming export missing or invalid" >&2
  exit 1
fi
if [[ ! -f "$EXPORT_PATH" ]] || ! rg -q '"id":7' "$EXPORT_PATH"; then
  cat "$audit_log" >&2
  echo "error: native result export file missing or invalid" >&2
  exit 1
fi
if find "$EXPORT_DIR" -name '*.tablerock-tmp-*' -print -quit | rg -q .; then
  echo "error: atomic export left a temporary file" >&2
  exit 1
fi

echo "shared Rust formatter, native copy, loaded export, and full streaming export gate passed"
