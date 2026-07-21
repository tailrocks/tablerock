# Native ordered sort headers

Date: 2026-07-22

## Requirement

Native object grids must make server-sort provenance visible at the columns,
including direction and priority for ordered multi-sort.

## Delivery

- Added a pure application-owned header projection from typed browse-sort DTOs.
- Active columns render ascending/descending direction plus one-based priority;
  unsorted columns preserve their database names exactly.
- Query-result grids remain unsorted; only object-tab Rust browse intent drives
  indicators. Swift still performs no SQL rendering or database work.
- Added isolated feature tests for unsorted, ascending, descending, and ordered
  multi-sort labels.

## Verification

```text
(cd native && swift build -c release)
# production package, including TableRockApp, compiled successfully

(cd native && swift test -c release --filter ColumnHeaderPresentationTests)
# local STOP: active developer directory is CommandLineTools and exposes no XCTest
```

Exact-commit hosted Xcode execution remains required before this evidence can
claim the test green.

## Remaining scope

Native show/hide, reorder, width/fit/format/reset controls and stable per-table
layout persistence remain open.

## Documentation and provenance

TablePro public documentation establishes only the broad expectation that grid
columns expose sort state and persistent layout controls. No TablePro source,
tests, identifiers, product text, assets, screenshots, layout measurements,
colors, or key bindings were copied or translated. Header expression derives
from TableRock's typed ordered-sort contract, native product requirements, and
direct tests.
