# 549 — Native Keychain resolution

Date: 2026-07-21

## Decision

Plan 021 checkpoint 12 now has an isolated native Keychain capability:

- `AppKeychainPort` stores, reads, and removes opaque persistent references;
- production `SystemKeychainPort` is the sole Security-framework adapter and
  scopes every query to the configured application/test namespace;
- explicit and fixture test roots derive different namespaces, while the
  unavailable default prevents tests from touching operator credentials;
- `BridgeProfileDraft` carries opaque reference bytes separately from password
  entry text; resolved values never enter persistence or bridge events;
- the engine accepts Keychain material only through an explicit native
  `KeychainReadPort`; non-native callers remain fail-closed.

The editor supports create/replace, duplicate requires a new credential,
connect/test/reconnect resolve only for that attempt, and profile deletion
removes its Keychain item. Cross-store cleanup failure is visible instead of
being silently treated as success.

## Verification

```text
cargo test -p tablerock-engine secret_resolution
cargo test -p tablerock-ffi --tests
cargo check -p tablerock-ffi --features bindgen-cli \
  --bin uniffi-bindgen-tablerock
./scripts/build-native-app.sh
./scripts/verify-native-accessibility.sh
```

All pass. The strict Swift 6 Release build proves Security-framework syntax;
the runtime accessibility gate proves the launched app remains operable.
Feature tests use a recording Keychain port; full execution remains assigned to
the canonical full-Xcode checkpoint because local Command Line Tools lack
Swift's `Testing` module.

## Generator repair

The committed UniFFI 0.32 binding fallback named a nonexistent crate feature
and binary. The crate now supplies the exact feature-gated helper expected by
the generation script, and generated Swift/header artifacts match the changed
record ABI.

## Provenance

Apple Keychain Services establishes encrypted small-secret storage. Apple's
item-return contract establishes that persistent references may be stored and
later matched to return password data:

- <https://developer.apple.com/documentation/security/keychain-services/>
- <https://developer.apple.com/documentation/security/item-return-result-keys>
- <https://developer.apple.com/documentation/security/ksecclassgenericpassword>

TablePro establishes the broad native saved-connection credential workflow
only. Copied source, tests, identifiers, assets, product text, screenshots,
layout measurements, colors, or key bindings: none.
