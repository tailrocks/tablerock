#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE="$REPO_ROOT/native/Sources/TableRockApp/TableRockApp.swift"
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
  'Label\("New group", systemImage: "folder.badge.plus"\)' \
  'Button\("Rename Group…"\)' \
  'Button\("Remove Group…", role: \.destructive\)' \
  'Label\("Manual Order"' \
  'Label\("Alphabetical"' \
  'Button\(profile.favorite \? "Remove Favorite" : "Add Favorite"\)' \
  'Button\("Move Up"\)' \
  'Button\("Move Down"\)' \
  'profile.connected \? "Connected" : "Disconnected"' \
  'Button\("Disconnect"\)' \
  'Connections in .* move to Ungrouped. No connection is deleted.'
do
  rg -q "$pattern" "$SOURCE" || {
    echo "error: missing native group contract: $pattern" >&2
    exit 1
  }
done

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
if ! rg -q '^PROFILE_GROUP_PROOF_PASSED ' "$audit_log"; then
  cat "$audit_log" >&2
  echo "error: native profile group runtime proof failed" >&2
  exit 1
fi

echo "native profile group structural and runtime gate passed"
