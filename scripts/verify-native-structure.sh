#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE="$REPO_ROOT/native/Sources/TableRockApp/TableRockApp.swift"
ENGINE="$REPO_ROOT/crates/tablerock-engine/src/relation_structure.rs"
FFI="$REPO_ROOT/crates/tablerock-ffi/src/bridge.rs"
APP="$REPO_ROOT/native/dist/TableRock.app"
EXECUTABLE="$APP/Contents/MacOS/TableRock"
CONTAINER="tablerock-native-structure-pg"
APP_PID=""

cleanup() {
  if [[ -n "$APP_PID" ]] && kill -0 "$APP_PID" 2>/dev/null; then
    kill "$APP_PID" 2>/dev/null || true
    wait "$APP_PID" 2>/dev/null || true
  fi
  docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
}
trap cleanup EXIT

for pattern in \
  'pub struct RelationStructureSnapshot' \
  'pub async fn load_relation_structure' \
  'pg_catalog.pg_get_indexdef' \
  'pg_catalog.pg_get_constraintdef'
do
  rg -q "$pattern" "$ENGINE" || { echo "error: missing shared structure contract: $pattern" >&2; exit 1; }
done
rg -q 'pub fn relation_structure' "$FFI" || {
  echo "error: missing FFI structure snapshot" >&2; exit 1;
}
for pattern in \
  'private struct ObjectStructureView' \
  'Text\("Structure"\).tag\("structure"\)' \
  'GroupBox\("Columns"\)' \
  '"Indexes"' \
  '"Constraints"'
do
  rg -q "$pattern" "$SOURCE" || { echo "error: missing native structure UI: $pattern" >&2; exit 1; }
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
docker exec "$CONTAINER" psql -U u -d db -v ON_ERROR_STOP=1 -c \
  "CREATE TABLE public.structure_probe (
     id bigint PRIMARY KEY,
     name text CONSTRAINT structure_probe_name_check CHECK (length(name) > 0),
     created_at timestamp NOT NULL DEFAULT now()
   );
   CREATE INDEX structure_probe_name_idx ON public.structure_probe (name);" >/dev/null

pgrep -f "^$EXECUTABLE$" >/dev/null && {
  echo "error: TableRock already running" >&2
  exit 1
}
"$REPO_ROOT/scripts/build-native-app.sh" >/dev/null
audit_log="$(mktemp "$REPO_ROOT/target/native-structure.XXXXXX")"
open -n -F --env TABLEROCK_FIXTURE_STRUCTURE=1 \
  --stdout "$audit_log" --stderr "$audit_log" "$APP"
for _ in $(seq 1 50); do
  APP_PID="$(pgrep -n -f "^$EXECUTABLE$" || true)"
  [[ -n "$APP_PID" ]] && break
  sleep 0.1
done
for _ in $(seq 1 100); do
  rg -q '^STRUCTURE_PROOF_' "$audit_log" && break
  sleep 0.1
done
if ! rg -q '^STRUCTURE_PROOF_PASSED ' "$audit_log"; then
  cat "$audit_log" >&2
  echo "error: native structure runtime proof failed" >&2
  exit 1
fi

echo "shared TUI/native PostgreSQL structure screen gate passed"
