#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE="$REPO_ROOT/native/Sources/TableRockApp/TableRockApp.swift"
BRIDGE="$REPO_ROOT/crates/tablerock-ffi/src/bridge.rs"
APP="$REPO_ROOT/native/dist/TableRock.app"
EXECUTABLE="$APP/Contents/MacOS/TableRock"
CONTAINER="tablerock-native-key-view-redis"
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
  'pub fn redis_key_view' \
  'decode_redis_catalog_key' \
  'redis_key_view_lines'
do
  rg -Fq "$pattern" "$BRIDGE" || { echo "error: missing Redis key bridge: $pattern" >&2; exit 1; }
done
for pattern in \
  'TABLEROCK_FIXTURE_REDIS_KEY_VIEW' \
  'private struct RedisKeyObjectView' \
  'REDIS_KEY_VIEW_PROOF_PASSED'
do
  rg -Fq "$pattern" "$SOURCE" || { echo "error: missing native Redis key view: $pattern" >&2; exit 1; }
done

docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
docker run -d --name "$CONTAINER" -p 6380:6379 redis:8.0 >/dev/null
for i in $(seq 1 30); do
  docker exec "$CONTAINER" redis-cli PING 2>/dev/null | rg -q '^PONG$' && break
  sleep 1
  [[ "$i" -eq 30 ]] && { echo "error: Redis not ready" >&2; exit 1; }
done
docker exec "$CONTAINER" redis-cli SET string-key value >/dev/null
docker exec "$CONTAINER" redis-cli RPUSH list-key a b >/dev/null
docker exec "$CONTAINER" redis-cli SADD set-key a b >/dev/null
docker exec "$CONTAINER" redis-cli ZADD zset-key 1 a 2 b >/dev/null
docker exec "$CONTAINER" redis-cli XADD stream-key '*' field value >/dev/null
hash_args=()
for i in $(seq 0 39); do hash_args+=("field-$i" "value-$i"); done
docker exec "$CONTAINER" redis-cli HSET hash-key "${hash_args[@]}" >/dev/null

pgrep -f "^$EXECUTABLE$" >/dev/null && {
  echo "error: TableRock already running" >&2
  exit 1
}
"$REPO_ROOT/scripts/build-native-app.sh" >/dev/null
audit_log="$(mktemp "$REPO_ROOT/target/native-redis-key-view.XXXXXX")"
open -n -F --env TABLEROCK_FIXTURE_REDIS_KEY_VIEW=1 \
  --stdout "$audit_log" --stderr "$audit_log" "$APP"
for _ in $(seq 1 50); do
  APP_PID="$(pgrep -n -f "^$EXECUTABLE$" || true)"
  [[ -n "$APP_PID" ]] && break
  sleep 0.1
done
for _ in $(seq 1 120); do
  rg -q '^REDIS_KEY_VIEW_PROOF_' "$audit_log" && break
  sleep 0.1
done
if ! rg -q '^REDIS_KEY_VIEW_PROOF_PASSED ' "$audit_log"; then
  cat "$audit_log" >&2
  echo "error: native Redis key view runtime proof failed" >&2
  exit 1
fi

echo "shared/native Redis key view gate passed"
