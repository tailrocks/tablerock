#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=lib/native-source-verifier.sh
source "$REPO_ROOT/scripts/lib/native-source-verifier.sh"
APP="$REPO_ROOT/native/dist/TableRock.app"
EXECUTABLE="$APP/Contents/MacOS/TableRock"
APP_PID=""

cleanup() {
  if [[ -n "$APP_PID" ]] && kill -0 "$APP_PID" 2>/dev/null; then
    kill "$APP_PID" 2>/dev/null || true
    wait "$APP_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

for pattern in \
  'Label\("Group", systemImage: "folder.badge.plus"\)' \
  'Button\("Rename Group…"\)' \
  'Button\("Remove Group…", role: \.destructive\)' \
  '"Manual Order",' \
  '"Alphabetical",' \
  'Button\(profile.favorite \? "Remove Favorite" : "Add Favorite"\)' \
  'Button\("Move Up"\)' \
  'Button\("Move Down"\)' \
  'guard profile.connected else \{ return "Disconnected" \}' \
  'Button\("Disconnect"\)' \
  'Button\("Check Health"\)' \
  'Button\("Reconnect"\)' \
  'planReconnect\(' \
  'case "authentication_stopped"' \
  'case "exhausted"' \
  'case "authentication_stopped": return "Authentication stopped"' \
  'EnvironmentSafetyBadge\(model: model\)' \
  'WorkbenchTabLabel\(title: tab.title, model: model\)' \
  'accessibilityLabel\("Environment ' \
  'Connections in .* move to Ungrouped. No connection is deleted.'
do
  native_source_has_regex "$pattern" || {
    echo "error: missing native group contract: $pattern" >&2
    exit 1
  }
done

native_production_source_has_regex 'reconnectSavedSessionWithSecret\(' || {
  echo "error: missing native live reconnect bridge contract" >&2
  exit 1
}

pgrep -f "^$EXECUTABLE$" >/dev/null && {
  echo "error: TableRock already running" >&2
  exit 1
}
"$REPO_ROOT/scripts/build-native-app.sh" >/dev/null
audit_log="$(mktemp "$REPO_ROOT/target/native-profile-groups.XXXXXX")"
open -n -F --env TABLEROCK_FIXTURE_PROFILE_GROUPS=1 \
  --stdout "$audit_log" --stderr "$audit_log" "$APP"
for _ in $(seq 1 50); do
  APP_PID="$(pgrep -n -f "^$EXECUTABLE$" || true)"
  [[ -n "$APP_PID" ]] && break
  sleep 0.1
done
for _ in $(seq 1 50); do
  rg -q '^PROFILE_GROUP_PROOF_' "$audit_log" && break
  sleep 0.1
done
if ! rg -q '^PROFILE_GROUP_PROOF_PASSED .*environment_surfaces=list_editor_context_tabs safety_surfaces=list_editor_context_tabs' "$audit_log"; then
  cat "$audit_log" >&2
  echo "error: native profile group runtime proof failed" >&2
  exit 1
fi

echo "native profile group structural and runtime gate passed"
