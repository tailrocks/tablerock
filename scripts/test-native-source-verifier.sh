#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=lib/native-source-verifier.sh
source "$REPO_ROOT/scripts/lib/native-source-verifier.sh"

fixture_root="$(mktemp -d "$REPO_ROOT/target/native-source-verifier.XXXXXX")"
cleanup() {
  rm -rf -- "$fixture_root"
}
trap cleanup EXIT

mkdir -p "$fixture_root/App" "$fixture_root/Bridge" "$fixture_root/Presentation"
printf '%s\n' 'ToolbarSpacer(.fixed)' >"$fixture_root/App/Root.swift"
printf '%s\n' 'BridgeWorkspaceTab conversion' >"$fixture_root/Bridge/Conversions.swift"
printf '%s\n' 'Redis [literal]' 'ToolbarSpacer(.fixed)' \
  >"$fixture_root/Presentation/Extracted.swift"

native_source_roots=("$fixture_root/App" "$fixture_root/Presentation")
native_production_source_roots=(
  "$fixture_root/App" "$fixture_root/Bridge" "$fixture_root/Presentation"
)

native_source_has_regex 'ToolbarSpacer\(\.fixed\)'
native_source_has_fixed 'Redis [literal]'
native_production_source_has_regex 'BridgeWorkspaceTab conversion'
native_production_source_has_fixed 'Redis [literal]'
if native_source_has_regex 'BridgeWorkspaceTab conversion'; then
  echo "error: presentation scope searched a bridge-only file" >&2
  exit 1
fi
if native_source_has_regex 'monolith-only sentinel'; then
  echo "error: native source regex helper returned a false positive" >&2
  exit 1
fi
if [[ "$(native_source_regex_count 'ToolbarSpacer\(\.fixed\)')" != "2" ]]; then
  echo "error: native source count did not span extracted files" >&2
  exit 1
fi

echo "native source verifier helper passed"
