# Plan 019 STOP — operator-provisioned toolchain

Date: 2026-07-18
Host: macOS with Command Line Tools only
Reconfirmed: 2026-07-18 @ commit `cc3aac8`+ (still blocked; software reconfirm evidence 298)

## Binding STOP conditions (from plan 019)

| Gate | Host state | Decision |
|------|------------|----------|
| Full Xcode for `xcodebuild -create-xcframework` | `xcode-select` → `/Library/Developer/CommandLineTools`; no `/Applications/Xcode.app`; `xcodebuild` requires Xcode | **STOP** packaging until full Xcode.app is selected |
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

## Software proof completed without certs (as of `c93b010`+)

| Area | Evidence |
|------|----------|
| Page codec + Swift PageV1 decode | 249, PageV1.swift |
| Facade + panic containment | 249, 250 |
| Conformance stubs + review tokens | 252, 254 |
| Real Docker open/probe/fetch ×3 engines | 253 |
| Universal lipo staticlib | 252 |
| Apply-by-handle + disconnect | 254 |
| Nest-safe Tokio `block_on` | runtime.rs |

## Non-blocked residual (optional polish)

- Persistence-backed `open(profile_id)` (OpenParams path is live today)
- Instruments leak pass (needs Xcode Instruments)
- UniFFI `@unchecked Sendable` Swift wrapper for plan 020