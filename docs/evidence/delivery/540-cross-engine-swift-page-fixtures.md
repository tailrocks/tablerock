# Cross-engine Swift PageV1 fixtures

Date: 2026-07-19

## Contract

SwiftPM now copies versioned PageV1 resources for PostgreSQL, ClickHouse, and
Redis into the bridge-test bundle. Each fixture is produced by the current Rust
`ResultPage::encode_v1` contract and proves the same Swift decoder preserves
engine identity, engine type, column name, and signed value display.

Fixtures are committed as reviewable hexadecimal bytes. A Rust integration
test independently reconstructs every source page, encodes it, decodes the hex
resource, and requires byte equality. Contract changes therefore cannot leave
the Swift golden resources silently stale.

## Evidence

- `swift test -c release`: 13 tests in three suites pass, including three
  parameterized cross-engine fixture cases.
- `cargo test -p tablerock-ffi --test page_fixture_resources`: pass.
- SwiftPM reports `Copying Fixtures`; tests resolve resources only through
  `Bundle.module`.

## Remaining boundary

Fixture breadth still needs NULL, empty, binary, structured, invalid,
truncated, multi-column columnar ordering, hostile body counts/offset overflow,
and future-version resources.

## Provenance

Fixtures come only from TableRock's Rust encoder and requirements. TablePro is
only a broad workflow reference; no source, tests, text, screenshots, layouts,
measurements, colors, assets, or key bindings were copied or translated.
