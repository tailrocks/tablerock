# Plan 019 STOP — operator-provisioned toolchain

Date: 2026-07-18
Host: macOS with Command Line Tools only

## Binding STOP conditions (from plan 019)

| Gate | Host state | Decision |
|------|------------|----------|
| Full Xcode for `xcodebuild -create-xcframework` | `xcode-select` → `/Library/Developer/CommandLineTools`; `xcodebuild` errors | **STOP** packaging until full Xcode.app is selected |
| Developer ID + notarization credentials | `security find-identity -v -p codesigning` → **0 valid identities**; `notarytool` not found | **STOP** distribution proof until operator provisions certs |

These are operator-provisioned gates, not architecture failures. The Rust
facade, page codec, UniFFI generation, and Swift proof harness already pass
on Command Line Tools alone.

## Already proven (no STOP)

- Page v1 encode/decode + pre-allocation bounds
- `tablerock-ffi` multi-thread runtime, panic containment, event/page facade
- Generated Swift bindings committed under `native/Generated`
- `tablerock-bridge-proof` Swift executable green
- Live `open(OpenParams)` connect path + unreachable reject
- Scripts: `scripts/generate-swift-bindings.sh`, `scripts/build-xcframework.sh`

## Resume when operator provides

1. Install/select full Xcode → run `./scripts/build-xcframework.sh`
2. Import Developer ID Application cert + App Store Connect API key / notary
   profile → sign, `notarytool submit --wait`, `stapler staple`
3. Clean-machine Gatekeeper transcript

## Non-blocked residual (can land without certs)

- Cross-adapter conformance suite (in-process vs bridge, three engines)
- Real-container Swift harness against Docker Postgres/CH/Redis
- Address UniFFI-generated `@unchecked Sendable` review
