# 570 — PageV1 hostile-boundary completion

Date: 2026-07-21

## Decision

The Swift PageV1 decoder now uses checked conversion and multiplication for
arena and cell/offset/bitmap sizes. Even a caller that deliberately supplies
maximal decode limits receives typed `sizeOverflow`; malformed page headers can
no longer trigger a Swift integer trap before bounds validation.

The canonical bridge suite now covers NULL, empty text, non-UTF-8 binary,
structured JSON, invalid, and truncated cells; row, column, arena, and column-
text limits; bad offsets; unsupported versions; representational overflow; and
1,000 repeated independent decodes. Existing committed fixtures cover the
PostgreSQL, ClickHouse, and Redis engine tags.

## Verification

```text
PATH=/Users/donbeave/.cargo/bin:$PATH ./scripts/build-native-app.sh
mise exec github:yonaskolb/XcodeGen@2.46.0 -- xcodegen generate --spec native/App/project.yml
```

The strict direct bridge/application compilation and complete app build pass.
The hosted canonical Xcode plan owns XCTest execution; hosted result is pending
for this checkpoint. No local XCTest claim is made because the selected Command
Line Tools SDK lacks XCTest.

## Remaining boundary

Hosted XCTest must pass before this checkpoint closes its canonical test-plan
gate. PageV1 version 1 remains the only accepted wire version; future versions
fail closed.

## Provenance

Implementation source: TableRock-owned PageV1 wire contract and hostile tests.

TablePro influence: none; this is serialization safety infrastructure.

Copied source, tests, identifiers, assets, strings, colors, geometry, layout
measurements, or key bindings: none.
