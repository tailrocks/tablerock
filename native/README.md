# Native / UniFFI package

Rust facade: `crates/tablerock-ffi`  
Generated Swift: `Generated/` (committed; regenerate with script)  
Proof executable: `Sources/BridgeProof`  
Page decode: `Sources/TableRockBridge/PageV1.swift`

## Build Rust library

```bash
cargo build -p tablerock-ffi --release
# optional universal staticlib (no full Xcode):
./scripts/build-universal-staticlib.sh
```

## Regenerate Swift bindings

```bash
# requires uniffi-bindgen 0.32.x on PATH
./scripts/generate-swift-bindings.sh
```

## Proof harness (Command Line Tools OK)

```bash
cargo build -p tablerock-ffi --release
cd native
DYLD_LIBRARY_PATH=../target/release swift run -c release tablerock-bridge-proof
```

## XCFramework + notarization (operator)

Requires **full Xcode.app** (not only CLT) and a **Developer ID Application**
identity + notary credentials:

```bash
./scripts/build-xcframework.sh
# then sign, notarytool submit --wait, stapler staple — see plan 019
```

Plan 020's locally runnable native vertical slice is complete. Plan 019's
Developer ID/notarization distribution gate remains blocked and is inherited by
Plan 021 release evidence; it does not prevent local development or verification.

```bash
./scripts/build-native-app.sh
open native/dist/TableRock.app
```
