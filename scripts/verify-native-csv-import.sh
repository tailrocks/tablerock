#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE="$REPO_ROOT/native/Sources/TableRockApp/TableRockApp.swift"
FILES="$REPO_ROOT/crates/tablerock-files/src/csv_import.rs"
FFI="$REPO_ROOT/crates/tablerock-ffi/src/bridge.rs"
APP="$REPO_ROOT/native/dist/TableRock.app"
EXECUTABLE="$APP/Contents/MacOS/TableRock"
CONTAINER="tablerock-native-import-pg"
APP_PID=""
CSV_PATH=""

cleanup() {
  if [[ -n "$APP_PID" ]] && kill -0 "$APP_PID" 2>/dev/null; then
    kill "$APP_PID" 2>/dev/null || true
    wait "$APP_PID" 2>/dev/null || true
  fi
  docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
  if [[ -n "$CSV_PATH" ]]; then rm -f "$CSV_PATH"; fi
}
trap cleanup EXIT

for pattern in \
  'pub fn read_csv_bounded' \
  'const MAX_CSV_COLUMNS: usize = 1_024' \
  'quote inside unquoted field' \
  "starts_with\(\['=', '\+', '-', '@'\]\)"
do
  rg -q "$pattern" "$FILES" || { echo "error: missing CSV safety contract: $pattern" >&2; exit 1; }
done
for pattern in \
  'pub fn preview_csv_import' \
  'pub fn stage_csv_import' \
  'CSV import requires a cached writable table' \
  'MutationPlan::new'
do
  rg -q "$pattern" "$FFI" || { echo "error: missing reviewed import bridge: $pattern" >&2; exit 1; }
done
for pattern in \
  'struct CsvImportSheet' \
  'Stage Reviewed Import' \
  'Apply Import' \
  'formula-like cells will be inserted as literal text' \
  'interactiveDismissDisabled'
do
  rg -q "$pattern" "$SOURCE" || { echo "error: missing native import UI: $pattern" >&2; exit 1; }
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
docker exec "$CONTAINER" psql -U u -d db -v ON_ERROR_STOP=1 \
  -c 'CREATE TABLE public.import_probe (id bigint PRIMARY KEY, name text NOT NULL)' >/dev/null

CSV_PATH="$(mktemp "$REPO_ROOT/target/native-import.XXXXXX.csv")"
printf 'id,name\n1,Ada\n2,=literal\n' >"$CSV_PATH"

pgrep -f "^$EXECUTABLE$" >/dev/null && {
  echo "error: TableRock already running" >&2
  exit 1
}
"$REPO_ROOT/scripts/build-native-app.sh" >/dev/null
audit_log="$(mktemp "$REPO_ROOT/target/native-csv-import.XXXXXX")"
open -n -F --env TABLEROCK_FIXTURE_CSV_IMPORT_PATH="$CSV_PATH" \
  --stdout "$audit_log" --stderr "$audit_log" "$APP"
for _ in $(seq 1 50); do
  APP_PID="$(pgrep -n -f "^$EXECUTABLE$" || true)"
  [[ -n "$APP_PID" ]] && break
  sleep 0.1
done
for _ in $(seq 1 100); do
  rg -q '^CSV_IMPORT_PROOF_' "$audit_log" && break
  sleep 0.1
done
if ! rg -q '^CSV_IMPORT_PROOF_PASSED ' "$audit_log"; then
  cat "$audit_log" >&2
  echo "error: native CSV import runtime proof failed" >&2
  exit 1
fi
rows="$(docker exec "$CONTAINER" psql -U u -d db -Atc 'SELECT count(*) FROM public.import_probe')"
[[ "$rows" == "2" ]] || { echo "error: expected 2 imported rows, got $rows" >&2; exit 1; }
literal="$(docker exec "$CONTAINER" psql -U u -d db -Atc "SELECT name FROM public.import_probe WHERE id = 2")"
[[ "$literal" == "=literal" ]] || { echo "error: formula-like text changed" >&2; exit 1; }

echo "native CSV preview, reviewed apply, and live PostgreSQL transaction gate passed"
