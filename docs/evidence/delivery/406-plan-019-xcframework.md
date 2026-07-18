# Plan 019 — universal XCFramework packaging

Date: 2026-07-19  
Host: macOS arm64, Xcode 26.6 (17F113)

## Checkpoint

Full Xcode became available. The packaging script initially passed separate
arm64 and x86_64 macOS frameworks to `xcodebuild -create-xcframework`, which
rejected them as equivalent library definitions. The script now creates one
universal macOS framework with `lipo`, then wraps that platform slice in an
XCFramework.

## Evidence

| Gate | Observation |
|------|-------------|
| `./scripts/build-xcframework.sh` | PASS |
| XCFramework identifier | `macos-arm64_x86_64` |
| `plutil` architectures | `arm64`, `x86_64` |
| `lipo -archs` | `x86_64 arm64` |
| `./scripts/build-native-app.sh` | PASS; ad-hoc-signed local `TableRock.app` built |

## Bounds and remaining STOP

The generated framework is intentionally unsigned: the host has zero valid
code-signing identities. Developer ID signing, hardened-runtime notarization,
stapling, and clean-machine Gatekeeper evidence remain operator-provisioned
STOP items. No release/distribution claim is made.
