# Native object-tab multi-sort

Date: 2026-07-22

## Requirement

Each native object tab must own independent, ordered server-sort intent. Swift
must never construct or concatenate SQL identifiers.

## Delivery

- Added the application-owned `WorkbenchBrowseSort` DTO and stored its ordered
  list on each `NativeObjectTab`.
- Added operable Add sort, direction-toggle, and remove controls. Every change
  reloads only the owning object tab and keeps the order visible to assistive
  technology.
- Added a typed UniFFI `BridgeBrowseSort` record and a sorted browse entrypoint.
  Rust rejects more than 16 keys, duplicate columns, unknown directions, and
  invalid identifiers.
- The existing engine `BrowsePlan` remains the only place intent becomes SQL;
  it quotes identifiers and preserves multi-key priority for PostgreSQL and
  ClickHouse. The original unsorted entrypoint remains compatible.

## Verification

```text
cargo nextest run -p tablerock-ffi --test conformance --locked \
  -E 'test(catalog_browse)'
# 2 passed

./scripts/generate-swift-bindings.sh
./scripts/build-native-app.sh
# generated bridge copies synchronized; strict Swift 6 app build passed
```

The named conformance test captures the request below the bridge and proves
both engines receive exactly ordered, Rust-rendered SQL:

```text
ORDER BY "created_at" DESC, "id" ASC LIMIT 500 OFFSET 0
```

It also proves invalid identifiers and directions fail before dispatch.

## Remaining scope

Typed server filters, durable object-tab restoration, column preferences, and
staged native edits remain open. This checkpoint does not claim full object-tab
or grid parity.

## Provenance

TablePro establishes that multi-column object browsing is a broad workflow.
No TablePro source, tests, identifiers, product text, assets, screenshots,
layout measurements, colors, or key bindings were copied or translated. The
contract and UI derive from TableRock's parity ledger, existing `BrowsePlan`,
Swift/AppKit conventions, and direct tests.
