# Plan 019 checkpoint — Swift bindings + proof harness

Date: 2026-07-18

## What landed

### Generated UniFFI Swift artifacts (committed)

Under `native/Generated/`:

- `tablerock_ffi.swift` — high-level Swift API
- `tablerock_ffiFFI.h` — C header
- `tablerock_ffiFFI.modulemap` / `module.modulemap` — module map for SPM

Mirrored into `native/Sources/TableRockBridge/` for the package target.

Regeneration:

```bash
cargo build -p tablerock-ffi --release
./scripts/generate-swift-bindings.sh
```

Uses `uniffi-bindgen` 0.32.0 library mode against
`target/release/libtablerock_ffi.dylib`.

### Swift package + proof executable

- `native/Package.swift` — Swift 6 package linking the cargo release dylib
- `native/Sources/BridgeProof` — CLT-friendly proof harness (no XCTest; full
  Xcode is not installed on this host — only Command Line Tools)

Checks proven:

```text
ok  panic_probe_contained
ok  open_params_rejects_unreachable_host
ok  runtime_ensure_destroy_idempotent
bridge-proof: all checks passed
```

Live `open(OpenParams)` now connects through engine adapters (PG/CH/Redis);
unreachable hosts return typed `Rejected { code: "connect" }`.

Command:

```bash
cargo build -p tablerock-ffi --release
cd native && DYLD_LIBRARY_PATH=../target/release swift run -c release tablerock-bridge-proof
```

### XCFramework script

`scripts/build-xcframework.sh` builds `aarch64-apple-darwin` +
`x86_64-apple-darwin` staticlibs and invokes `xcodebuild -create-xcframework`.

**Blocked on this host**: active developer directory is Command Line Tools
only (`xcode-select` points at `/Library/Developer/CommandLineTools`);
`xcodebuild` requires full Xcode. Notarization also blocked (0 codesigning
identities).

## Residual

1. Full Xcode + XCFramework packaging on an operator machine with Xcode.app
2. Developer ID sign / notarize / staple
3. Live `open(OpenParams)` connect for three engines + conformance suite
4. Review UniFFI-generated `@unchecked Sendable` on `TableRockBridge` against
   the strict-concurrency gate (generated; not hand-authored)

## Provenance

- UniFFI 0.32.0 Swift guide (sync facade; library-mode generation)
- Apple XCFramework packaging docs (script only until Xcode available)
