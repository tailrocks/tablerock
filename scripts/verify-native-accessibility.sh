#!/usr/bin/env bash
# Static structural gate for native custom-control accessibility and ownership.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE="$REPO_ROOT/native/Sources/TableRockApp/TableRockApp.swift"

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

echo "native accessibility structural gate passed"
