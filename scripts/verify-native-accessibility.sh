#!/usr/bin/env bash
# Static structural gate for native custom-control accessibility and ownership.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE="$REPO_ROOT/native/Sources/TableRockApp/TableRockApp.swift"
APP_BUNDLE="$REPO_ROOT/native/dist/TableRock.app"
APP_EXECUTABLE="$APP_BUNDLE/Contents/MacOS/TableRock"
APP_PID=""

cleanup() {
  if [[ -n "$APP_PID" ]] && kill -0 "$APP_PID" 2>/dev/null; then
    kill "$APP_PID" 2>/dev/null || true
    wait "$APP_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

require() {
  local pattern="$1"
  local description="$2"
  if ! rg -q "$pattern" "$SOURCE"; then
    echo "error: missing $description" >&2
    exit 1
  fi
}

forbid() {
  local pattern="$1"
  local description="$2"
  if rg -q "$pattern" "$SOURCE"; then
    echo "error: forbidden $description" >&2
    exit 1
  fi
}

require 'setAccessibilityLabel\("Database catalog"\)' 'catalog outline label'
require 'Catalog (object|group)' 'catalog row semantic labels'
require 'setAccessibilityLabel\("Query results"\)' 'result table label'
require 'setAccessibilityValue\(value\)' 'result cell accessible value'
require 'setAccessibilityLabel\("SQL editor"\)' 'SQL editor label'
require 'accessibilityLabel\("Refresh catalog"\)' 'catalog refresh label'
require 'Label\("Run Query"' 'Run toolbar/menu label'
require 'Label\("Cancel Query"' 'Cancel toolbar/menu label'
require 'Fixture ·' 'appearance evidence marker'
require '\.buttonStyle\(\.glassProminent\)' 'glass-prominent primary toolbar action'
require 'backgroundColor = \.textBackgroundColor' 'opaque editor/grid content surfaces'
if [[ "$(rg -c 'ToolbarSpacer\(\.fixed\)' "$SOURCE")" -lt 2 ]]; then
  echo "error: missing toolbar glass-cluster separators" >&2
  exit 1
fi

forbid 'NSVisualEffectView' 'custom visual-effect material'
forbid '\.blur\(' 'custom blur'
forbid 'toolbarBackground|[A-Za-z]+Material' 'custom toolbar or material background'
forbid 'DispatchQueue' 'GCD ownership bypass'
forbid 'ObservableObject|@Published|@StateObject|@EnvironmentObject' 'legacy observation stack'

if pgrep -f "^$APP_EXECUTABLE$" >/dev/null; then
  echo "error: TableRock is already running; close it before accessibility proof" >&2
  exit 1
fi
"$REPO_ROOT/scripts/build-native-app.sh" >/dev/null
audit_log="$(mktemp "$REPO_ROOT/target/native-accessibility.XXXXXX")"
open -n -F --env TABLEROCK_FIXTURE_ACCESSIBILITY_AUDIT=1 \
  --stdout "$audit_log" --stderr "$audit_log" "$APP_BUNDLE"
for _ in $(seq 1 50); do
  APP_PID="$(pgrep -n -f "^$APP_EXECUTABLE$" || true)"
  [[ -n "$APP_PID" ]] && break
  sleep 0.1
done
for _ in $(seq 1 50); do
  rg -q '^CATALOG_STATE_PROOF_' "$audit_log" && break
  sleep 0.1
done
if ! rg -q '^ACCESSIBILITY_PROOF_PASSED ' "$audit_log"; then
  cat "$audit_log" >&2
  echo "error: native accessibility runtime proof failed" >&2
  exit 1
fi
if ! rg -q '^CATALOG_EXPANSION_REQUEST key=node:' "$audit_log"; then
  cat "$audit_log" >&2
  echo "error: catalog expansion did not dispatch refresh intent" >&2
  exit 1
fi
if ! rg -q '^CATALOG_STATE_PROOF_PASSED ' "$audit_log"; then
  cat "$audit_log" >&2
  echo "error: catalog loading/stale state runtime proof failed" >&2
  exit 1
fi

echo "native accessibility structural and runtime gate passed"
