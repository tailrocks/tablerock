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
CAPTURE_SCOPE="${2:-full}"
CURRENT_PID=""
CAPTURE_MANIFEST="$OUT_DIR/CAPTURES.tsv"

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
find "$OUT_DIR" -maxdepth 1 -type f \( \
  -name '*.png' -o -name '*.log' -o -name 'CAPTURES.tsv' -o \
  -name 'MANIFEST.tsv' -o -name 'SHA256SUMS' \
\) -delete
printf 'file\tconcept\tsurface\tpresentation\tappearance\taccessibility\tengine\tfixture\twindow_size\tactivity\texpected_points\tpixel_width\tpixel_height\tlaunch_arguments\n' \
  >"$CAPTURE_MANIFEST"

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
  local engine="${5:-postgresql}"
  local fixture="${6:-populated}"
  local window_size="${7:-typical}"
  local activity="${8:-active}"
  local presentation="${9:-standard}"
  local name="${concept}__${surface}__${appearance}__${accessibility}__${engine}__${fixture}__${window_size}__${activity}"
  if [[ "$presentation" != "standard" ]]; then
    name="${name}__${presentation}"
  fi
  local log="$OUT_DIR/$name.log"
  local image="$OUT_DIR/$name.png"
  local expected_points=""
  local -a launch_arguments=(
    --concept "$concept"
    --surface "$surface"
    --appearance "$appearance"
    --accessibility "$accessibility"
    --engine "$engine"
    --fixture "$fixture"
    --window-size "$window_size"
    --capture
  )

  case "$window_size" in
    minimum) expected_points="1280x760" ;;
    typical) expected_points="1440x900" ;;
    expanded) expected_points="1720x1040" ;;
    *) echo "error: unknown window size $window_size" >&2; exit 1 ;;
  esac

  if [[ "$activity" == "inactive" ]]; then
    launch_arguments+=(--inactive)
  elif [[ "$activity" != "active" ]]; then
    echo "error: unknown activity $activity" >&2
    exit 1
  fi

  if [[ "$presentation" != "standard" ]]; then
    launch_arguments+=(--presentation "$presentation")
  fi

  open -n -F --stdout "$log" --stderr "$log" "$APP_BUNDLE" --args \
    "${launch_arguments[@]}"

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

  sleep 0.5

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

  local pixel_width
  local pixel_height
  local launch_text
  pixel_width="$(sips -g pixelWidth "$image" | awk '/pixelWidth/ {print $2}')"
  pixel_height="$(sips -g pixelHeight "$image" | awk '/pixelHeight/ {print $2}')"
  printf -v launch_text '%q ' "${launch_arguments[@]}"
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$(basename "$image")" "$concept" "$surface" "$presentation" "$appearance" \
    "$accessibility" "$engine" "$fixture" "$window_size" "$activity" \
    "$expected_points" "$pixel_width" "$pixel_height" "${launch_text% }" \
    >>"$CAPTURE_MANIFEST"

  kill "$CURRENT_PID"
  wait "$CURRENT_PID" 2>/dev/null || true
  CURRENT_PID=""
  echo "captured $name"
}

if [[ "$CAPTURE_SCOPE" == "full" ]]; then
  for concept in "${concepts[@]}"; do
    for surface in "${surfaces[@]}"; do
      capture "$concept" "$surface" light system
    done
    for surface in data-grid sql-results; do
      capture "$concept" "$surface" dark system
    done
    capture "$concept" data-grid light system postgresql populated typical inactive
  done

  for accessibility in reduce-transparency increase-contrast reduce-motion; do
    capture native-workbench data-grid light "$accessibility"
  done

  capture native-workbench data-grid light system postgresql populated minimum active
  capture native-workbench data-grid light system postgresql populated expanded active

  for fixture in empty loading connection-error large-result long-identifiers selected-cell pending-change destructive-review; do
    capture native-workbench data-grid light system postgresql "$fixture" typical active
  done

  capture native-workbench data-grid light system clickhouse populated typical active
  capture native-workbench data-grid light system redis populated typical active
  MATRIX_DESCRIPTION='25 light surfaces + 10 dark work surfaces + 5 inactive windows + 3 accessibility previews + 2 alternate sizes + 8 scenarios + 2 alternate engines'
elif [[ "$CAPTURE_SCOPE" == "refined" ]]; then
  capture native-workbench connections light system
  capture native-workbench connections light system postgresql populated typical active connection-sheet
  capture native-workbench setup light system
  capture native-workbench data-grid light system
  capture native-workbench data-grid dark system
  capture native-workbench data-grid light system postgresql populated typical active structure
  capture native-workbench data-grid light system postgresql populated typical active safe-edit
  capture native-workbench sql-results light system
  capture native-workbench sql-results dark system
  capture native-workbench sql-results light system postgresql populated typical active query-error
  capture native-workbench sql-results light system postgresql populated typical active query-history
  capture native-workbench change-review light system postgresql pending-change
  capture native-workbench data-grid light system postgresql populated typical active safe-review
  capture native-workbench data-grid light system postgresql destructive-review
  for fixture in empty loading connection-error selected-cell; do
    capture native-workbench data-grid light system postgresql "$fixture"
  done
  for accessibility in reduce-transparency increase-contrast reduce-motion; do
    capture native-workbench data-grid light "$accessibility"
  done
  capture native-workbench data-grid light system postgresql populated minimum
  capture native-workbench data-grid light system postgresql populated expanded
  capture native-workbench data-grid light system postgresql populated typical inactive
  capture native-workbench data-grid light system clickhouse
  capture native-workbench data-grid light system redis
  MATRIX_DESCRIPTION='Native Workbench refined flows + sheets + error/states + light/dark + accessibility + sizing + inactive + three engines'
else
  echo "error: unknown capture scope $CAPTURE_SCOPE (expected full or refined)" >&2
  exit 1
fi

find "$OUT_DIR" -name '*.log' -empty -delete
shasum -a 256 "$OUT_DIR"/*.png >"$OUT_DIR/SHA256SUMS"

{
  printf 'revision\t%s\n' "$(git -C "$REPO_ROOT" rev-parse HEAD)"
  printf 'host\t%s\n' "$(sw_vers -productVersion) ($(sw_vers -buildVersion))"
  printf 'xcode\t%s\n' "$(xcodebuild -version | tr '\n' ' ')"
  printf 'sdk\t%s\n' "$(xcrun --sdk macosx --show-sdk-version)"
  printf 'matrix\t%s\n' "$MATRIX_DESCRIPTION"
  printf 'capture_count\t%s\n' "$(find "$OUT_DIR" -maxdepth 1 -name '*.png' | wc -l | tr -d ' ')"
} >"$OUT_DIR/MANIFEST.tsv"

echo "Design Lab captures written to $OUT_DIR"
