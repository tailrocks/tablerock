#!/usr/bin/env bash
# Record a bounded native grid scroll with Time Profiler, RSS, and leak evidence.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_BUNDLE="$REPO_ROOT/native/dist/TableRock.app"
APP_EXECUTABLE="$APP_BUNDLE/Contents/MacOS/TableRock"
OUT_DIR="${1:-$(mktemp -d "$REPO_ROOT/target/native-performance.XXXXXX")}"
APP_PID=""
RSS_PID=""

cleanup() {
  if [[ -n "$RSS_PID" ]] && kill -0 "$RSS_PID" 2>/dev/null; then
    kill "$RSS_PID" 2>/dev/null || true
    wait "$RSS_PID" 2>/dev/null || true
  fi
  if [[ -n "$APP_PID" ]] && kill -0 "$APP_PID" 2>/dev/null; then
    kill "$APP_PID" 2>/dev/null || true
    wait "$APP_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

command -v xcrun >/dev/null
if pgrep -f "^$APP_EXECUTABLE$" >/dev/null; then
  echo "error: TableRock is already running; close it before capture" >&2
  exit 1
fi
mkdir -p "$OUT_DIR"
"$REPO_ROOT/scripts/build-native-app.sh" >/dev/null

open -n -F \
  --env TABLEROCK_FIXTURE_GRID_ROWS=10000 \
  --env TABLEROCK_FIXTURE_AUTOSCROLL=1 \
  --stdout "$OUT_DIR/app.log" --stderr "$OUT_DIR/app.log" \
  "$APP_BUNDLE"

for _ in $(seq 1 50); do
  APP_PID="$(pgrep -n -f "^$APP_EXECUTABLE$" || true)"
  [[ -n "$APP_PID" ]] && break
  sleep 0.1
done
if [[ -z "$APP_PID" ]]; then
  echo "error: TableRock process did not start" >&2
  exit 1
fi

for _ in $(seq 1 50); do
  rg -q '^PERF_FIXTURE_READY rows=10000 columns=8 ' "$OUT_DIR/app.log" && break
  sleep 0.1
done
if ! rg -q '^PERF_FIXTURE_READY rows=10000 columns=8 ' "$OUT_DIR/app.log"; then
  echo "error: performance fixture did not become ready" >&2
  exit 1
fi

(
  while kill -0 "$APP_PID" 2>/dev/null; do
    ps -o rss= -p "$APP_PID" | tr -d ' '
    sleep 0.1
  done
) >"$OUT_DIR/rss-kb.txt" &
RSS_PID="$!"

xcrun xctrace record --template 'Time Profiler' --attach "$APP_PID" \
  --time-limit 6s --output "$OUT_DIR/scroll.trace" --no-prompt

kill "$RSS_PID" 2>/dev/null || true
wait "$RSS_PID" 2>/dev/null || true
RSS_PID=""

for _ in $(seq 1 30); do
  rg -q '^PERF_SCROLL_DONE rows=10000 ' "$OUT_DIR/app.log" && break
  sleep 0.1
done
rg -q '^PERF_SCROLL_DONE rows=10000 ' "$OUT_DIR/app.log"

# Run leak inspection only after RSS sampling; inspection itself perturbs RSS.
leaks "$APP_PID" >"$OUT_DIR/leaks.txt" 2>&1 || true
xcrun xctrace export --input "$OUT_DIR/scroll.trace" --toc \
  --output "$OUT_DIR/scroll-toc.xml" >/dev/null

fixture_metric="$(rg '^PERF_FIXTURE_READY ' "$OUT_DIR/app.log")"
scroll_metric="$(rg '^PERF_SCROLL_DONE ' "$OUT_DIR/app.log")"
max_rss_kb="$(awk 'NF && $1 > max { max = $1 } END { print max + 0 }' "$OUT_DIR/rss-kb.txt")"
leak_summary="$(rg -m1 'leaks for .*total leaked bytes' "$OUT_DIR/leaks.txt" || true)"
trace_bytes="$(du -sk "$OUT_DIR/scroll.trace" | awk '{ print $1 * 1024 }')"
{
  echo "$fixture_metric"
  echo "$scroll_metric"
  echo "PERF_MAX_RSS_KB $max_rss_kb"
  echo "PERF_TRACE_BYTES $trace_bytes"
  echo "PERF_LEAK_SCAN ${leak_summary:-unavailable}"
  xcrun xctrace version
  sw_vers
  system_profiler SPHardwareDataType | rg 'Model Name|Model Identifier|Chip|Memory'
} >"$OUT_DIR/metrics.txt"

cat "$OUT_DIR/metrics.txt"
echo "evidence: $OUT_DIR"
