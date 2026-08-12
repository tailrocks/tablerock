#!/usr/bin/env bash
# Capture the deterministic Design Lab gate matrix without changing system
# appearance or accessibility preferences.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NATIVE="$REPO_ROOT/native"
DERIVED_DATA="$REPO_ROOT/target/design-lab-derived-data"
APP_BUNDLE="$DERIVED_DATA/Build/Products/Debug/TableRockDesignLab.app"
APP_EXECUTABLE="$APP_BUNDLE/Contents/MacOS/TableRockDesignLab"
OUT_DIR="${1:-$REPO_ROOT/docs/evidence/design-lab/captures}"
CURRENT_PID=""

concepts=(native-workbench query-studio column-observatory grid-canvas change-desk)
surfaces=(connections setup data-grid sql-results change-review)

cleanup() {
  if [[ -n "$CURRENT_PID" ]] && kill -0 "$CURRENT_PID" 2>/dev/null; then
    kill "$CURRENT_PID" 2>/dev/null || true
    wait "$CURRENT_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

if [[ ! -x "$APP_EXECUTABLE" ]]; then
  xcodegen generate --spec "$NATIVE/App/project.yml"
  xcodebuild \
    -project "$NATIVE/App/TableRock.xcodeproj" \
    -scheme TableRockDesignLab \
    -configuration Debug \
    -destination 'platform=macOS' \
    -derivedDataPath "$DERIVED_DATA" \
    build
fi

if pgrep -f "^$APP_EXECUTABLE" >/dev/null; then
  echo "error: TableRock Design Lab is already running" >&2
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

capture() {
  local concept="$1"
  local surface="$2"
  local appearance="$3"
  local accessibility="$4"
  local name="${concept}__${surface}__${appearance}__${accessibility}"
  local log="$OUT_DIR/$name.log"
  local image="$OUT_DIR/$name.png"

  open -n -F --stdout "$log" --stderr "$log" "$APP_BUNDLE" --args \
    --concept "$concept" \
    --surface "$surface" \
    --appearance "$appearance" \
    --accessibility "$accessibility" \
    --capture

  CURRENT_PID=""
  for _ in $(seq 1 40); do
    CURRENT_PID="$(pgrep -n -f "^$APP_EXECUTABLE" || true)"
    [[ -n "$CURRENT_PID" ]] && break
    sleep 0.1
  done

  local window_id=""
  for _ in $(seq 1 40); do
    if [[ -n "$CURRENT_PID" ]]; then
      window_id="$(window_id_for_pid "$CURRENT_PID")"
    fi
    [[ -n "$window_id" ]] && break
    sleep 0.15
  done

  if [[ -z "$CURRENT_PID" || -z "$window_id" ]]; then
    echo "error: no Design Lab window for $name" >&2
    exit 1
  fi

  local captured=""
  for _ in $(seq 1 20); do
    if screencapture -x -o -l "$window_id" "$image" 2>/dev/null \
        && [[ -s "$image" ]]; then
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
}

for concept in "${concepts[@]}"; do
  for surface in "${surfaces[@]}"; do
    capture "$concept" "$surface" light system
  done
  for surface in data-grid sql-results; do
    capture "$concept" "$surface" dark system
  done
done

for accessibility in reduce-transparency increase-contrast reduce-motion; do
  capture native-workbench data-grid light "$accessibility"
done

find "$OUT_DIR" -name '*.log' -empty -delete
shasum -a 256 "$OUT_DIR"/*.png >"$OUT_DIR/SHA256SUMS"

{
  printf 'revision\t%s\n' "$(git -C "$REPO_ROOT" rev-parse HEAD)"
  printf 'host\t%s\n' "$(sw_vers -productVersion) ($(sw_vers -buildVersion))"
  printf 'xcode\t%s\n' "$(xcodebuild -version | tr '\n' ' ')"
  printf 'sdk\t%s\n' "$(xcrun --sdk macosx --show-sdk-version)"
  printf 'matrix\t25 light surfaces + 10 dark work surfaces + 3 accessibility previews\n'
} >"$OUT_DIR/MANIFEST.tsv"

echo "Design Lab captures written to $OUT_DIR"
