#!/usr/bin/env bash
# Prove the native Design Lab is dependency-isolated, deterministic, and
# independently buildable. It intentionally does not build the production app.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NATIVE="$REPO_ROOT/native"
SOURCE="$NATIVE/Sources/TableRockDesignLab"
DERIVED_DATA="$REPO_ROOT/target/design-lab-derived-data"
BUILD_LOG="$REPO_ROOT/target/design-lab-xcodebuild.log"
UI_TEST_LOG="$REPO_ROOT/target/design-lab-ui-test.log"

if rg -n \
  'import (TableRockBridge|TableRockFeature|Network)|TableRockBridge|TableRockFeature|tablerock_ffi|UniFFI|BridgeModel|WorkbenchBackend|URLSession' \
  "$SOURCE"; then
  echo "error: Design Lab references a forbidden production or I/O symbol" >&2
  exit 1
fi

package_dependency_count="$(
  (cd "$NATIVE" && swift package dump-package) |
    jq '[.targets[] | select(.name == "TableRockDesignLab")][0].dependencies | length'
)"
if [[ "$package_dependency_count" != "0" ]]; then
  echo "error: Swift Package Design Lab target has dependencies" >&2
  exit 1
fi

xcodegen generate --spec "$NATIVE/App/project.yml"

swift build --package-path "$NATIVE" --product TableRockDesignLab
swift test --package-path "$NATIVE" --filter TableRockDesignLabTests

xcodebuild \
  -project "$NATIVE/App/TableRock.xcodeproj" \
  -scheme TableRockDesignLab \
  -configuration Debug \
  -destination 'platform=macOS' \
  -derivedDataPath "$DERIVED_DATA" \
  build | tee "$BUILD_LOG"

if ! rg -q "Target 'TableRockDesignLab'.*\(no dependencies\)" "$BUILD_LOG"; then
  echo "error: Xcode did not prove a dependency-free Design Lab graph" >&2
  exit 1
fi

xcodebuild \
  -project "$NATIVE/App/TableRock.xcodeproj" \
  -scheme TableRockDesignLab \
  -configuration Debug \
  -destination 'platform=macOS' \
  -derivedDataPath "$DERIVED_DATA" \
  -only-testing:TableRockDesignLabUITests/TableRockDesignLabUITests \
  test | tee "$UI_TEST_LOG"

echo "Design Lab isolation verified"
