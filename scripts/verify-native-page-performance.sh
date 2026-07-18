#!/usr/bin/env bash
# Profile bounded Swift PageV1 decode over a real 500-row UniFFI page.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NATIVE="$REPO_ROOT/native"
BUILD="$NATIVE/.build-direct"
OUT_DIR="${1:-$(mktemp -d "$REPO_ROOT/target/native-page-performance.XXXXXX")}"
CONTAINER="tablerock-pg-page-perf"

cleanup() {
  docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
}
trap cleanup EXIT

mkdir -p "$OUT_DIR"
"$REPO_ROOT/scripts/build-native-app.sh" >/dev/null
swiftc -swift-version 6 -strict-concurrency=complete -warnings-as-errors \
  -I "$BUILD" -I "$NATIVE/Generated" -Xcc -I -Xcc "$NATIVE/Generated" \
  -target arm64-apple-macos26.0 "$NATIVE/Sources/BehaviorProof/main.swift" \
  "$BUILD/tablerock_ffi.o" "$BUILD/PageV1.o" \
  -L "$REPO_ROOT/target/release" -ltablerock_ffi -framework Foundation \
  -o "$BUILD/BehaviorProof"

cleanup
docker run -d --name "$CONTAINER" \
  -e POSTGRES_PASSWORD=secret -e POSTGRES_USER=u -e POSTGRES_DB=db \
  -p 5433:5432 postgres:18.4-alpine >/dev/null
for attempt in $(seq 1 30); do
  docker exec "$CONTAINER" pg_isready -U u >/dev/null 2>&1 && break
  sleep 1
  if [[ "$attempt" == 30 ]]; then
    echo "error: PostgreSQL performance fixture did not become ready" >&2
    exit 1
  fi
done

query="SELECT i, repeat('x', 64) AS payload FROM generate_series(1, 500) AS i"
environment=(
  "DYLD_LIBRARY_PATH=$REPO_ROOT/target/release"
  TABLEROCK_ENGINE=postgresql
  TABLEROCK_PORT=5433
  TABLEROCK_DB=db
  "TABLEROCK_QUERY=$query"
  TABLEROCK_DECODE_BENCH=2000
)

xcrun xctrace record --template 'Time Profiler' --time-limit 10s \
  --output "$OUT_DIR/page-decode.trace" --no-prompt --launch -- \
  /usr/bin/env "${environment[@]}" "$BUILD/BehaviorProof" \
  >"$OUT_DIR/xctrace.log" 2>&1
xcrun xctrace export --input "$OUT_DIR/page-decode.trace" --toc \
  --output "$OUT_DIR/page-decode-toc.xml" >/dev/null

/usr/bin/env "${environment[@]}" "$BUILD/BehaviorProof" \
  >"$OUT_DIR/benchmark.log" 2>&1
metric="$(rg '^PERF_PAGE_DECODE ' "$OUT_DIR/benchmark.log")"
trace_bytes="$(du -sk "$OUT_DIR/page-decode.trace" | awk '{ print $1 * 1024 }')"
{
  echo "$metric"
  echo "PERF_TRACE_BYTES $trace_bytes"
  xcrun xctrace version
  sw_vers
  system_profiler SPHardwareDataType | rg 'Model Name|Model Identifier|Chip|Memory'
} >"$OUT_DIR/metrics.txt"

cat "$OUT_DIR/metrics.txt"
echo "evidence: $OUT_DIR"
