#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_SOURCE="$REPO_ROOT/native/Sources/TableRockApp"
FEATURE_SOURCE="$REPO_ROOT/native/Sources/TableRockFeature"
BRIDGE_SOURCE="$REPO_ROOT/native/Sources/TableRockBridge"
PRESENTATION_SOURCE="$REPO_ROOT/native/Sources/TableRockPresentation"
PROJECT_SPEC="$REPO_ROOT/native/App/project.yml"
DEVELOPMENT_FIXTURE_SYMBOLS="$REPO_ROOT/scripts/native-development-fixture-symbols.txt"
APP_DEVELOPMENT_SUPPORT="$APP_SOURCE/DevelopmentSupport"
FEATURE_DEVELOPMENT_SUPPORT="$FEATURE_SOURCE/AppConfigurationDevelopmentSupport.swift"

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

if rg -n '\bBridgeModel\b' "$APP_SOURCE" "$REPO_ROOT/native/Tests/TableRockAppTests"; then
  echo "error: retired BridgeModel name returned" >&2
  exit 1
fi

if [[ -e "$APP_SOURCE/TableRockApp.swift" ]]; then
  echo "error: retired TableRockApp.swift monolith returned" >&2
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

development_condition = "$(inherited) TABLEROCK_DEVELOPMENT_SUPPORT"
%w[TableRockFeature TableRock].each do |name|
  configs = targets.fetch(name).fetch("settings").fetch("configs")
  unless configs.dig("Debug", "SWIFT_ACTIVE_COMPILATION_CONDITIONS") == development_condition &&
      configs.dig("Test Release", "SWIFT_ACTIVE_COMPILATION_CONDITIONS") == development_condition &&
      !configs.fetch("Release", {}).key?("SWIFT_ACTIVE_COMPILATION_CONDITIONS")
    abort "error: #{name} development-support compilation conditions drifted"
  end
end
RUBY

actual_fixture_symbols="$(mktemp "$REPO_ROOT/target/native-fixture-symbols.XXXXXX")"
cleanup() {
  rm -f -- "$actual_fixture_symbols"
}
trap cleanup EXIT

for support_file in "$APP_DEVELOPMENT_SUPPORT/DevelopmentSupport.swift" \
  "$FEATURE_DEVELOPMENT_SUPPORT"; do
  if [[ "$(sed -n '/[^[:space:]]/{p;q;}' "$support_file")" != \
      '#if TABLEROCK_DEVELOPMENT_SUPPORT' ]]; then
    echo "error: development support is not compile-time guarded: $support_file" >&2
    exit 1
  fi
done

if rg -n -g '!**/DevelopmentSupport/**' -g '!AppConfigurationDevelopmentSupport.swift' \
  'TABLEROCK_FIXTURE_|ScriptedWorkbenchBackend' "${production_roots[@]}"; then
  echo "error: fixture or scripted-backend code escaped DevelopmentSupport" >&2
  exit 1
fi

rg -o --no-filename 'TABLEROCK_FIXTURE_[A-Z0-9_]+' \
  "$APP_DEVELOPMENT_SUPPORT" "$FEATURE_DEVELOPMENT_SUPPORT" \
  | sort -u >"$actual_fixture_symbols"
added="$(comm -13 "$DEVELOPMENT_FIXTURE_SYMBOLS" "$actual_fixture_symbols")"
if [[ -n "$added" ]]; then
  printf 'error: new development fixture route lacks review:\n%s\n' "$added" >&2
  exit 1
fi
removed="$(comm -23 "$DEVELOPMENT_FIXTURE_SYMBOLS" "$actual_fixture_symbols")"
if [[ -n "$removed" ]]; then
  printf 'error: development fixture inventory is stale:\n%s\n' "$removed" >&2
  exit 1
fi

"$REPO_ROOT/scripts/test-native-source-verifier.sh"

echo "native source ownership gate passed"
