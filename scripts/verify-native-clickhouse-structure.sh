#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE="$REPO_ROOT/native/Sources/TableRockApp/TableRockApp.swift"
ENGINE="$REPO_ROOT/crates/tablerock-engine/src/clickhouse.rs"
APP="$REPO_ROOT/native/dist/TableRock.app"
EXECUTABLE="$APP/Contents/MacOS/TableRock"
CONTAINER="tablerock-native-structure-ch"
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
  'system.columns' \
  'system.tables' \
  '{db:String}' \
  '{tbl:String}'
do
  rg -Fq "$pattern" "$ENGINE" || { echo "error: missing safe ClickHouse metadata query: $pattern" >&2; exit 1; }
done
for pattern in \
  'TABLEROCK_FIXTURE_CLICKHOUSE_STRUCTURE' \
  'GroupBox("Engine facts")' \
  'CLICKHOUSE_STRUCTURE_PROOF_PASSED'
do
  rg -Fq "$pattern" "$SOURCE" || { echo "error: missing native ClickHouse structure projection: $pattern" >&2; exit 1; }
done

docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
docker run -d --name "$CONTAINER" \
  -e CLICKHOUSE_USER=u -e CLICKHOUSE_PASSWORD=secret -e CLICKHOUSE_DB=db \
  -p 8122:8123 clickhouse/clickhouse-server:25.8 >/dev/null
for i in $(seq 1 30); do
  docker exec "$CONTAINER" clickhouse-client --user u --password secret \
    --database db --query 'SELECT 1' >/dev/null 2>&1 && break
  sleep 1
  [[ "$i" -eq 30 ]] && { echo "error: ClickHouse not ready" >&2; exit 1; }
done
docker exec "$CONTAINER" clickhouse-client --user u --password secret --database db \
  --multiquery --query \
  "CREATE TABLE db.structure_probe (
     id UInt64 COMMENT 'identity',
     name Nullable(String),
     created_at DateTime DEFAULT now()
   ) ENGINE = MergeTree
   PARTITION BY toYYYYMM(created_at)
   ORDER BY id;" >/dev/null

pgrep -f "^$EXECUTABLE$" >/dev/null && {
  echo "error: TableRock already running" >&2
  exit 1
}
"$REPO_ROOT/scripts/build-native-app.sh" >/dev/null
audit_log="$(mktemp "$REPO_ROOT/target/native-clickhouse-structure.XXXXXX")"
open -n -F --env TABLEROCK_FIXTURE_CLICKHOUSE_STRUCTURE=1 \
  --stdout "$audit_log" --stderr "$audit_log" "$APP"
for _ in $(seq 1 50); do
  APP_PID="$(pgrep -n -f "^$EXECUTABLE$" || true)"
  [[ -n "$APP_PID" ]] && break
  sleep 0.1
done
for _ in $(seq 1 100); do
  rg -q '^CLICKHOUSE_STRUCTURE_PROOF_' "$audit_log" && break
  sleep 0.1
done
if ! rg -q '^CLICKHOUSE_STRUCTURE_PROOF_PASSED ' "$audit_log"; then
  cat "$audit_log" >&2
  echo "error: native ClickHouse structure runtime proof failed" >&2
  exit 1
fi

echo "shared TUI/native ClickHouse structure screen gate passed"
