#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_SOURCE="$REPO_ROOT/native/Sources/TableRockApp"
PRESENTATION_SOURCE="$REPO_ROOT/native/Sources/TableRockPresentation"
production_roots=("$APP_SOURCE" "$PRESENTATION_SOURCE")
content_sources=(
  "$PRESENTATION_SOURCE/CatalogGrid.swift"
  "$PRESENTATION_SOURCE/ObjectBrowseSurfaces.swift"
  "$PRESENTATION_SOURCE/ObjectWorkbenchView.swift"
  "$PRESENTATION_SOURCE/QueryWorkbenchView.swift"
  "$PRESENTATION_SOURCE/ResultSurfaces.swift"
  "$PRESENTATION_SOURCE/SqlTextEditor.swift"
)

if rg -n '\.glassEffect\(|GlassEffectContainer' "${production_roots[@]}"; then
  echo "error: custom production glass lacks an approved functional owner" >&2
  exit 1
fi

if rg -n '\.(blur|shadow)\(' "${production_roots[@]}"; then
  echo "error: production contains handcrafted blur or shadow material" >&2
  exit 1
fi

if rg -n '\.(ultraThinMaterial|thinMaterial|regularMaterial|thickMaterial|barMaterial)\b' \
    "${production_roots[@]}"; then
  echo "error: production contains explicit simulated material" >&2
  exit 1
fi

if rg -n '\.buttonStyle\(\.glass(Prominent)?\)' "${content_sources[@]}"; then
  echo "error: CONTENT surface owns glass" >&2
  exit 1
fi

for required in \
  'grid.backgroundColor = .textBackgroundColor' \
  'editor.backgroundColor = .textBackgroundColor' \
  'scroll.backgroundColor = .textBackgroundColor'
do
  if ! rg -Fq "$required" "${content_sources[@]}"; then
    echo "error: opaque native content contract missing: $required" >&2
    exit 1
  fi
done

if ! rg -q '\.buttonStyle\(\.glass(Prominent)?\)' "$PRESENTATION_SOURCE"; then
  echo "error: system Liquid Glass controls are absent from functional chrome" >&2
  exit 1
fi

echo "native Liquid Glass ownership gate passed"
