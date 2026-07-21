# Native saved filter presets

Date: 2026-07-22

## Requirement

Native object tabs need profile- and object-scoped Save/Load filter presets
with the same Rust-owned behavior and Turso library as the TUI.

## Structural correction

The preset type, validation, JSON codec, and lookup library previously lived
inside `tablerock-tui`. That presentation ownership prevented native reuse and
allowed client behavior to diverge. The contract now lives in
`tablerock-core`; the TUI re-exports the same condition, preset, and library
types rather than owning a second implementation.

## Delivery

- Added redacted typed UniFFI preset records and list/upsert methods. Rust
  derives profile, schema, and object identity from session and opaque catalog
  handles; Swift cannot choose another persistence scope.
- Enforced the existing safe-name grammar, 32-condition maximum, 1,024-byte
  identifiers, 64-KiB values/raw fragment, operator allow-list, and value
  arity before persistence.
- Added native preset name, Save, and Load controls with stable accessibility
  identifiers. Loading rekeys presentation identities, restores typed filters
  plus raw-WHERE, then re-browses.
- Object reload failure now retains the last successful grid instead of
  destructively blanking useful state before I/O succeeds.

## Verification

```text
cargo nextest run -p tablerock-core -p tablerock-tui -p tablerock-ffi --locked
# 497 passed, 5 skipped

cargo clippy -p tablerock-core -p tablerock-tui -p tablerock-cli \
  -p tablerock-ffi --all-targets --locked -- -D warnings
# clean

./scripts/generate-swift-bindings.sh
./scripts/build-native-app.sh
# generated bridge synchronized; strict Swift 6 direct app build passed

xcrun swiftc -parse native/Tests/TableRockAppUITests/TableRockAppUITests.swift
# parsed
```

The FFI conformance suite opens a saved profile, derives its cached PostgreSQL
schema/table identity, persists a typed preset, reads it back exactly, and
rejects an unsafe name. The scripted XCUITest now saves, clears, reloads, and
observes restored raw mode through real controls. Hosted Xcode execution is
pending and is not claimed green here.

## Remaining scope

Durable object-tab restoration, native resident-page quick filtering,
column-type-driven editors, and column layout persistence remain open.

## Documentation and provenance

TablePro establishes only the broad saved-filter workflow. No TablePro source,
tests, identifiers, product text, assets, screenshots, layout measurements,
colors, or key bindings were copied or translated. Behavior derives from the
existing TableRock TUI/core contract, persistence schema 13, product
requirements, and direct tests.
