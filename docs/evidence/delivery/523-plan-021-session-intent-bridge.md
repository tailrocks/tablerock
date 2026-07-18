# Plan 021 — typed session-intent bridge

Date: 2026-07-19

## Contract

UniFFI now exposes typed per-profile session-intent get, put, and delete
operations. An intent contains database/schema context, selected tab index,
and tab title/statement text only. Rust validates at least one and at most 64
tabs, selected-index range, 256-byte database/schema/title bounds, and 1 MiB
statement text per tab before serializing the existing persistence JSON.

Loads parse and validate persisted JSON inside Rust before projection to Swift.
Result pages, cells, operation state, and pending writes have no typed field and
the persistence boundary continues rejecting their reserved keys. This makes
intent-only restoration structural rather than dependent on Swift discipline.

## Evidence

| Gate | Result |
|---|---|
| typed two-tab put/get equality | pass |
| selected tab and database/schema round trip | pass |
| delete then load-none | pass |
| focused UniFFI persistence conformance | pass |

## Remaining boundary

Native independent query/object/result tab presentation and `WindowGroup`
restoration are not claimed here. This bridge is their required Rust-owned
prerequisite.

## Documentation source

Context7 quota remained exhausted. Current platform behavior was checked
against Apple Developer Documentation: `WindowGroup` persists its bound value
for restoration, and macOS can gather its windows as native tabs. AppKit
`NSWindow` tabbing identifiers group related windows.

## Provenance

TablePro was used only to confirm the broad concepts of independent workbench
tabs and restoring editor intent without results. No source, tests, text,
screenshots, layouts, measurements, colors, assets, or key bindings were copied
or translated.
