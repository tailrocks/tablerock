# Plan 019 checkpoint — conformance expansion + universal staticlib

Date: 2026-07-18

## Conformance suite (`cargo test -p tablerock-ffi`)

| Case | Proof |
|------|--------|
| Page byte-for-byte (PG/CH/Redis stubs) | `bridge_page_bytes_match_in_process_encode_all_engines` |
| Command validation | unknown intent + stale revision rejected |
| Event ordering / future cursor | sequences increase; `FutureCursor` on ahead cursor |
| Cancel pending op | `cancel_pending_operation_requests_cancel` + cancel-active shutdown |
| Shutdown after completed work | graceful, second submit rejected |
| Redaction | `OpenParams` Debug redacts password |
| Oversized page decode | arena limit before body allocation |
| No per-cell UniFFI export | generated Swift has no `cell`/`fetchCell`; facade returns `Vec<u8>` only |

```text
cargo test -p tablerock-ffi
# 12 passed (4 suites)
```

Live Docker three-engine paths remain on the engine real-server CI matrix;
bridge stubs cover the contract surface for all three engines.

## Universal staticlib (no full Xcode)

```bash
./scripts/build-universal-staticlib.sh
lipo -info target/universal/libtablerock_ffi.a
# Architectures in the fat file: ... are: x86_64 arm64
```

Artifact: `target/universal/libtablerock_ffi.a` (~109 MB).  
XCFramework via `xcodebuild -create-xcframework` still needs full Xcode.app
(see evidence 251).

## Generated-artifact determinism

Two consecutive `uniffi-bindgen generate --library … -l swift` runs on the
same release dylib produce identical outputs (`diff -ru` empty).

## UniFFI `@unchecked Sendable` note

Generated Swift marks `TableRockBridge` and internal `UniffiHandleMap` as
`@unchecked Sendable`. This is UniFFI 0.32 scaffolding, not hand-authored
TableRock code. Rust side serializes all facade state behind `Mutex`; Swift
callers must still hop off `MainActor` for polling (documented
native-macos-path). Residual strict-concurrency polish may wrap the handle
in plan 020 without changing the Rust facade.

## Still operator-blocked

- Developer ID sign / notarize / staple (0 identities on host)
- Full Xcode XCFramework packaging (CLT only)

## Residual non-blocked (next)

- Real-container bridge tests (optional; engine suite already covers drivers)
- Review-token apply through bridge handle (mutation registry already in facade)
- Instruments leak pass on a machine with Xcode Instruments
