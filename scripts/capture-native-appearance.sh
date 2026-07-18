#!/usr/bin/env bash
# Capture the native Phase-13 appearance matrix without changing system prefs.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_BUNDLE="$REPO_ROOT/native/dist/TableRock.app"
APP_EXECUTABLE="$APP_BUNDLE/Contents/MacOS/TableRock"
OUT_DIR="${1:-$REPO_ROOT/target/native-appearance}"
CURRENT_PID=""

cleanup() {
  if [[ -n "$CURRENT_PID" ]] && kill -0 "$CURRENT_PID" 2>/dev/null; then
    kill "$CURRENT_PID" 2>/dev/null || true
    wait "$CURRENT_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

if [[ ! -x "$APP_EXECUTABLE" ]]; then
  "$REPO_ROOT/scripts/build-native-app.sh"
fi
if pgrep -f "^$APP_EXECUTABLE$" >/dev/null; then
  echo "error: TableRock is already running; close it before capture" >&2
  exit 1
fi
mkdir -p "$OUT_DIR"

window_id_for_pid() {
  local app_pid="$1"
  swift -e '
    import CoreGraphics
    import Foundation
    let expectedPID = Int(CommandLine.arguments[1])!
    let rows = CGWindowListCopyWindowInfo(
      [.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID
    )! as! [[String: Any]]
    var best: (number: Int, area: Int)?
    for row in rows {
      guard row[kCGWindowOwnerPID as String] as? Int == expectedPID,
            let number = row[kCGWindowNumber as String] as? Int,
            let layer = row[kCGWindowLayer as String] as? Int,
            let bounds = row[kCGWindowBounds as String] as? [String: Any],
            let width = bounds["Width"] as? Int,
            let height = bounds["Height"] as? Int,
            layer == 0 else { continue }
      let candidate = (number, width * height)
      if best == nil || candidate.1 > best!.area { best = candidate }
    }
    if let best { print(best.number) }
  ' "$app_pid"
}

for scheme in light dark; do
  for contrast in 0 1; do
    for transparency in 0 1; do
      name="${scheme}-contrast${contrast}-transparency${transparency}"
      open -n -F \
        --env "TABLEROCK_FIXTURE_APPEARANCE=$scheme" \
        --env "TABLEROCK_FIXTURE_CONTRAST=$contrast" \
        --env "TABLEROCK_FIXTURE_REDUCE_TRANSPARENCY=$transparency" \
        --stdout "$OUT_DIR/$name.log" --stderr "$OUT_DIR/$name.log" \
        "$APP_BUNDLE"

      CURRENT_PID=""
      for _ in $(seq 1 30); do
        CURRENT_PID="$(pgrep -n -f "^$APP_EXECUTABLE$" || true)"
        [[ -n "$CURRENT_PID" ]] && break
        sleep 0.1
      done
      window_id=""
      for _ in $(seq 1 30); do
        window_id="$(window_id_for_pid "$CURRENT_PID")"
        [[ -n "$window_id" ]] && break
        sleep 0.2
      done
      if [[ -z "$CURRENT_PID" || -z "$window_id" ]]; then
        echo "error: no TableRock window for $name" >&2
        exit 1
      fi
      captured=""
      for _ in $(seq 1 20); do
        if screencapture -x -o -l "$window_id" "$OUT_DIR/$name.png" 2>/dev/null \
            && [[ -s "$OUT_DIR/$name.png" ]]; then
          captured="1"
          break
        fi
        sleep 0.1
      done
      if [[ -z "$captured" ]]; then
        echo "error: window pixels unavailable for $name" >&2
        exit 1
      fi
      kill "$CURRENT_PID"
      wait "$CURRENT_PID" 2>/dev/null || true
      CURRENT_PID=""
      echo "captured $name"
    done
  done
done

shasum -a 256 "$OUT_DIR"/*.png >"$OUT_DIR/SHA256SUMS"
