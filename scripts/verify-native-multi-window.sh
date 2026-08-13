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
  'private let application = NativeApplicationModel\(\)' \
  'WindowGroup\(for: UUID.self\)' \
  'application\.disablesWindowRestoration \? \.disabled : \.automatic' \
  'ProcessInfo\.processInfo\.environment\["TABLEROCK_TEST_MODE"\] == "1"' \
  'disablesWindowRestoration = false' \
  '@State private var model: WorkbenchPresentationStore' \
  'window.tabbingIdentifier = "tablerock-workbench"' \
  'window.tabbingMode = \.preferred' \
  'return client === otherClient'
do
  native_source_has_regex "$pattern" || {
    echo "error: missing native multi-window contract: $pattern" >&2
    exit 1
  }
done

pgrep -f "^$EXECUTABLE$" >/dev/null && {
  echo "error: TableRock already running" >&2
  exit 1
}
"$REPO_ROOT/scripts/build-native-app.sh" >/dev/null
audit_log="$(mktemp "$REPO_ROOT/target/native-multi-window.XXXXXX")"
open -n -F --env TABLEROCK_FIXTURE_MULTI_WINDOW=1 \
  --stdout "$audit_log" --stderr "$audit_log" "$APP"
for _ in $(seq 1 50); do
  APP_PID="$(pgrep -n -f "^$EXECUTABLE$" || true)"
  [[ -n "$APP_PID" ]] && break
  sleep 0.1
done
for _ in $(seq 1 50); do
  rg -q '^MULTI_WINDOW_PROOF_' "$audit_log" && break
  sleep 0.1
done
if ! rg -q '^MULTI_WINDOW_PROOF_PASSED ' "$audit_log"; then
  cat "$audit_log" >&2
  echo "error: native multi-window runtime proof failed" >&2
  exit 1
fi

echo "native multi-window structural and runtime gate passed"
