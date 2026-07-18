# Plan 019 checkpoint — page v1 codec + tablerock-ffi facade

Date: 2026-07-18

## What landed

### Page encoding (tablerock-core)

- `ResultPage::encode_v1` / `decode_v1` — versioned columnar byte-arena wire
  format (magic `TRP1`, encoding_version = 1).
- Envelope is validated against `PageLimits` **before** large buffer allocation
  on decode (arena / row / column / column-text bounds).
- Round-trip unit tests in `crates/tablerock-core/tests/page.rs`.
- Oversized-arena rejection tested without allocating the declared size.

### UniFFI facade crate (`crates/tablerock-ffi`)

- New workspace member; `crate-type = ["lib", "staticlib", "cdylib"]`.
- Dependency adoption: `uniffi = 0.32.0` (MPL-2.0), exact pin; `tokio` gains
  `rt-multi-thread` for the multi-thread runtime owned by the facade.
- Synchronous coarse API: `create`, `ensure_runtime`, `open`, `submit`,
  `pump`, `next_events`, `fetch_page`, `cancel`, `shutdown`, `destroy_runtime`,
  `panic_probe`.
- Multi-thread Tokio runtime: explicit ensure + idempotent destroy.
- `catch_unwind` at every exported entry → `BridgeError::ContainedPanic`.
- Pages leave the facade only as `encode_v1` bytes (no per-cell calls).
- `OpenParams` Debug redacts password.
- Rust-only `open_driver_session` injects `DriverSession` for unit/conformance.
- Live `open(OpenParams)` connect wiring deferred to the Swift harness
  checkpoint (returns typed rejection until then).
- `EngineService::core_mut` added for dynamic scope registration at the
  bridge boundary.

## Verification

```text
cargo test -p tablerock-core --test page
cargo test -p tablerock-ffi
```

Both green on this checkpoint.

## Residual (plan 019 remaining)

1. Generated Swift package + deterministic regeneration proof
2. XCFramework packaging script (`aarch64` + `x86_64`, lipo/create-xcframework)
3. Swift 6 proof harness under `native/`
4. Cross-adapter conformance suite (in-process + bridge, three engines)
5. Developer ID sign / notarize / staple — **blocked** until operator
   provisions codesigning identity (none on this host: `security
   find-identity -v -p codesigning` → 0 identities)

## Provenance

- UniFFI: https://mozilla.github.io/uniffi-rs/ (sync coarse facade; avoid
  generated async per native-macos-path.md / shared-client-contract.md)
- Page format: TableRock-owned columnar arena; Arrow explicitly rejected
