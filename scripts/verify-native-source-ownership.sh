#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_SOURCE="$REPO_ROOT/native/Sources/TableRockApp"
FEATURE_SOURCE="$REPO_ROOT/native/Sources/TableRockFeature"
BRIDGE_SOURCE="$REPO_ROOT/native/Sources/TableRockBridge"
PRESENTATION_SOURCE="$REPO_ROOT/native/Sources/TableRockPresentation"
PROJECT_SPEC="$REPO_ROOT/native/App/project.yml"
FIXTURE_DEBT="$REPO_ROOT/scripts/native-release-fixture-debt.txt"
FIXTURE_PATH_DEBT="$REPO_ROOT/scripts/native-release-fixture-path-debt.txt"

production_roots=("$APP_SOURCE" "$FEATURE_SOURCE" "$BRIDGE_SOURCE")
if [[ -d "$PRESENTATION_SOURCE" ]]; then
  production_roots+=("$PRESENTATION_SOURCE")
fi

if rg -n '^import TableRockDesignLab$' "${production_roots[@]}"; then
  echo "error: production imports the Design Lab" >&2
  exit 1
fi

if rg -n \
  '^import (TableRockBridge|TableRockPresentation|TableRockDesignLab|tablerock_ffiFFI|SwiftUI|AppKit)$' \
  "$FEATURE_SOURCE"; then
  echo "error: TableRockFeature crosses its stable-contract boundary" >&2
  exit 1
fi

if rg -n '^import (TableRockPresentation|TableRockDesignLab|SwiftUI|AppKit)$' \
  "$BRIDGE_SOURCE"; then
  echo "error: TableRockBridge imports presentation code" >&2
  exit 1
fi

if [[ -d "$PRESENTATION_SOURCE" ]] && rg -n \
  '(^import (TableRockBridge|TableRockDesignLab|tablerock_ffiFFI|Network|Security)$)|tablerock_ffi|URLSession' \
  "$PRESENTATION_SOURCE"; then
  echo "error: TableRockPresentation crosses its bridge-neutral boundary" >&2
  exit 1
fi

if rg -n -g '!verify-native-source-ownership.sh' \
  'SOURCE=.*TableRockApp\.swift' "$REPO_ROOT/scripts"; then
  echo "error: native verifier is coupled to the former monolith file" >&2
  exit 1
fi

monolith_lines="$(wc -l <"$APP_SOURCE/TableRockApp.swift" | tr -d ' ')"
if [[ "$monolith_lines" -gt 12231 ]]; then
  echo "error: TableRockApp.swift grew beyond the frozen 12,231-line baseline" >&2
  exit 1
fi

package_json="$(swift package dump-package --package-path "$REPO_ROOT/native")"
if [[ "$(jq '[.targets[] | select(.name == "TableRockDesignLab")][0].dependencies | length' \
  <<<"$package_json")" != "0" ]]; then
  echo "error: Swift Package Design Lab target has dependencies" >&2
  exit 1
fi
if jq -e '
  [.targets[] | select(.name == "TableRockApp")][0].dependencies
  | any((.byName?[0] // .target?[0] // .product?[0] // "") == "TableRockDesignLab")
' <<<"$package_json" >/dev/null; then
  echo "error: Swift Package production app depends on Design Lab" >&2
  exit 1
fi
if [[ "$(jq -r '
  [.targets[] | select(.name == "TableRockBridge")][0].dependencies
  | map(.byName?[0] // .target?[0] // .product?[0] // "")
  | sort | join("\n")
' <<<"$package_json")" != $'TableRockFeature\ntablerock_ffiFFI' ]]; then
  echo "error: Swift Package Bridge dependencies drifted" >&2
  exit 1
fi

ruby - "$PROJECT_SPEC" <<'RUBY'
require "yaml"

path = ARGV.fetch(0)
targets = YAML.load_file(path).fetch("targets")
lab_dependencies = targets.fetch("TableRockDesignLab").fetch("dependencies", [])
abort "error: Xcode Design Lab target has dependencies" unless lab_dependencies.empty?

app_dependencies = targets.fetch("TableRock").fetch("dependencies", [])
if app_dependencies.any? { |entry| entry["target"] == "TableRockDesignLab" }
  abort "error: Xcode production app depends on Design Lab"
end

bridge_dependencies = targets.fetch("TableRockBridge").fetch("dependencies", [])
bridge_target_names = bridge_dependencies.map { |entry| entry["target"] }.compact
unless bridge_target_names == ["TableRockFeature"]
  abort "error: Xcode Bridge must depend only on the Feature target"
end

if targets.key?("TableRockPresentation")
  presentation_dependencies = targets.fetch("TableRockPresentation").fetch("dependencies", [])
  names = presentation_dependencies.map { |entry| entry["target"] }.compact
  unless names == ["TableRockFeature"]
    abort "error: Xcode Presentation target must depend only on TableRockFeature"
  end
  unless app_dependencies.any? { |entry| entry["target"] == "TableRockPresentation" }
    abort "error: Xcode production app does not depend on Presentation"
  end
end
RUBY

actual_fixture_symbols="$(mktemp "$REPO_ROOT/target/native-fixture-symbols.XXXXXX")"
unexpected_fixture_files="$(mktemp "$REPO_ROOT/target/native-fixture-files.XXXXXX")"
cleanup() {
  rm -f -- "$actual_fixture_symbols" "$unexpected_fixture_files"
}
trap cleanup EXIT

rg -o --no-filename 'TABLEROCK_FIXTURE_[A-Z0-9_]+' "${production_roots[@]}" \
  | sort -u >"$actual_fixture_symbols"
added="$(comm -13 "$FIXTURE_DEBT" "$actual_fixture_symbols")"
if [[ -n "$added" ]]; then
  printf 'error: new Release fixture debt is forbidden:\n%s\n' "$added" >&2
  exit 1
fi

rg -l 'TABLEROCK_FIXTURE_|ScriptedWorkbenchBackend' "${production_roots[@]}" \
  | sed "s#^$REPO_ROOT/##" \
  | sort -u \
  >"$unexpected_fixture_files" || true
unexpected_fixture_paths="$(comm -13 "$FIXTURE_PATH_DEBT" "$unexpected_fixture_files")"
if [[ -n "$unexpected_fixture_paths" ]]; then
  echo "error: existing Release fixture debt escaped its frozen source files:" >&2
  printf '%s\n' "$unexpected_fixture_paths" >&2
  exit 1
fi

"$REPO_ROOT/scripts/test-native-source-verifier.sh"

echo "native source ownership gate passed"
