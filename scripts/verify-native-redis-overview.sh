#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE="$REPO_ROOT/native/Sources/TableRockApp/TableRockApp.swift"
ADAPTER="$REPO_ROOT/crates/tablerock-engine/src/adapter.rs"
APP="$REPO_ROOT/native/dist/TableRock.app"
EXECUTABLE="$APP/Contents/MacOS/TableRock"
CONTAINER="tablerock-native-overview-redis"
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
  'TABLEROCK_FIXTURE_REDIS_OVERVIEW' \
  'private struct RedisOverviewSheet' \
  'REDIS_OVERVIEW_PROOF_PASSED'
do
  rg -Fq "$pattern" "$SOURCE" || { echo "error: missing native Redis overview: $pattern" >&2; exit 1; }
done
for pattern in \
  'redis_info_lines' \
  'unavailable (INFO field absent)' \
  'instantaneous_ops_per_sec'
do
  rg -Fq "$pattern" "$ADAPTER" || { echo "error: missing shared Redis overview fact: $pattern" >&2; exit 1; }
done

docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
docker run -d --name "$CONTAINER" -p 6380:6379 redis:8.0 >/dev/null
for i in $(seq 1 30); do
  docker exec "$CONTAINER" redis-cli PING 2>/dev/null | rg -q '^PONG$' && break
  sleep 1
  [[ "$i" -eq 30 ]] && { echo "error: Redis not ready" >&2; exit 1; }
done
for i in $(seq 1 30); do
  nc -z 127.0.0.1 6380 2>/dev/null && break
  sleep 0.2
  [[ "$i" -eq 30 ]] && { echo "error: Redis host port not ready" >&2; exit 1; }
done
docker exec "$CONTAINER" redis-cli SET overview-probe value >/dev/null

pgrep -f "^$EXECUTABLE$" >/dev/null && {
  echo "error: TableRock already running" >&2
  exit 1
}
"$REPO_ROOT/scripts/build-native-app.sh" >/dev/null
audit_log="$(mktemp "$REPO_ROOT/target/native-redis-overview.XXXXXX")"
open -n -F --env TABLEROCK_FIXTURE_REDIS_OVERVIEW=1 \
  --stdout "$audit_log" --stderr "$audit_log" "$APP"
for _ in $(seq 1 50); do
  APP_PID="$(pgrep -n -f "^$EXECUTABLE$" || true)"
  [[ -n "$APP_PID" ]] && break
  sleep 0.1
done
for _ in $(seq 1 120); do
  rg -q '^REDIS_OVERVIEW_PROOF_' "$audit_log" && break
  sleep 0.1
done
if ! rg -q '^REDIS_OVERVIEW_PROOF_PASSED ' "$audit_log"; then
  cat "$audit_log" >&2
  echo "error: native Redis overview runtime proof failed" >&2
  exit 1
fi

echo "shared/native Redis overview gate passed"
