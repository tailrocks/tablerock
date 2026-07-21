# Native raw-WHERE browse intent

Date: 2026-07-22

## Requirement

Native table-like object tabs need the same explicit raw-WHERE escape hatch as
the TUI without moving SQL composition or validation into Swift.

## Delivery

- Extended the typed browse entrypoint with an optional raw fragment.
- Rust rejects fragments above 65,536 UTF-8 bytes, delegates empty/parameter-
  collision validation to `BrowsePlan`, and parenthesizes the accepted fragment
  while composing it with parameterized typed filters.
- Added native Apply/Clear controls, a visible active-state announcement, and
  stable accessibility identifiers. Draft and active state remain independent
  per object tab.
- Extended the existing sort/filter XCUITest to type, apply, observe, and clear
  raw-WHERE through real controls.

## Verification

```text
cargo nextest run -p tablerock-ffi --test conformance --locked
# 17 passed; PostgreSQL and ClickHouse statement captures include the
# parenthesized raw fragment and preserve two separate typed parameters

cargo clippy -p tablerock-ffi --all-targets --locked -- -D warnings
# clean

./scripts/generate-swift-bindings.sh
./scripts/build-native-app.sh
# bindings regenerated; strict Swift 6 direct app build passed

xcrun swiftc -parse native/Tests/TableRockAppUITests/TableRockAppUITests.swift
# parsed
```

Hosted Xcode UI execution is pending. This checkpoint does not claim that gate
before its exact-main run completes. Local SwiftPM XCTest cannot run because
the selected developer directory exposes CommandLineTools without XCTest; the
hosted full-Xcode checkpoint remains authoritative.

## Remaining scope

Native saved filter presets, durable restoration, column-type-driven editors,
and resident-page quick filtering remain open.

## Documentation and provenance

TablePro establishes only the broad raw-filter workflow. No TablePro source,
tests, identifiers, product text, assets, screenshots, layout measurements,
colors, or key bindings were copied or translated. Behavior derives from the
existing TableRock `BrowsePlan`, product requirements, and direct tests.
