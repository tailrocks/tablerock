# Native Redis key object views

Date: 2026-07-19

## Contract

Native Redis key tabs pass only the session ID, opaque catalog node ID, and a
bounded collection offset through UniFFI. Rust validates node ownership,
decodes the reversible catalog identity, and delegates to the shared Redis
adapter. Swift neither receives key bytes nor assembles Redis commands.

The shared adapter returns bounded display lines for String, Hash, List, Set,
Sorted Set, and Stream keys, including type and TTL facts. Scanned Hash, Set,
and Sorted Set views expose explicit continuation. These are typed display
views, not editable result pages; binary key identity remains exact below
presentation while displayed values use the existing bounded rendering
contract.

## Evidence

- Redis 8.0 live native fixture proves all six known key kinds through opaque
  catalog handles.
- A 40-field Hash proves first-page rendering and explicit second-page append;
  the visible audit reaches `field-39 = value-39`.
- UniFFI conformance proves a Redis catalog node resolves through the opaque
  bridge and returns its typed view.
- Native Swift 6 build and shared/native Redis key-view gate pass.

## Remaining boundary

Redis key mutation, namespace grouping, logical-database context switching,
key filtering, and catalog pagination continuation remain. Structured binary
cell inspection would require a later result-page contract rather than this
display-line projection.

## Provenance

TablePro established only the broad typed key-inspection workflow. No source,
tests, text, screenshots, layouts, measurements, colors, assets, or key
bindings were copied or translated. Implementation follows this repository's
shared Redis contracts, redis-rs documentation, and direct Redis tests.
