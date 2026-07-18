# Plan 021 — profile favorites and ordering

Date: 2026-07-19

## Contract

Migration `0015-profile-group-ordering.sql` gives every registered group an
explicit manual or alphabetical sort mode. Favorite mutation and exact manual
reordering run through the serialized persistence actor with expected-revision
checks. Reordering validates the complete group membership, rejects duplicate
or stale inputs, writes a dense order, and advances every member revision in
one transaction. This removes partial and lost-update ordering states.

UniFFI exposes typed group settings, revision-safe favorite mutation, and a
bounded exact-order request. Its profile listing consumes every cursor page,
so native search and grouping do not silently omit profiles after the first
100. The TUI loader does the same. Both clients present favorites first;
alphabetical groups then compare names, while manual groups preserve saved
order. TUI targets now include the profile database as well as host and port.

## Client behavior

SwiftUI rows expose favorite toggles and manual move controls. Group menus
select Manual or Alphabetical ordering and show the active mode. The TUI action
bar and keyboard cycle expose the same operations. Manual movement cannot cross
the favorite boundary; changing favorite status is the explicit way to move
between those partitions.

## Evidence

| Gate | Result |
|---|---|
| migration 15 backfill and group sort-mode persistence | pass |
| favorite stale-revision rejection | pass |
| exact reorder membership, duplicate, and stale-revision rejection | pass |
| persistence suite | pass; 37 tests |
| UniFFI conformance, including 101-profile pagination | pass; 17 tests, 5 ignored |
| TUI suite | pass; 315 tests |
| CLI suite and PTY lifecycle | pass; 53 tests, 7 ignored |
| native group structural/runtime gate | pass |
| native editor and accessibility structural/runtime gates | pass |

Live connection-health resolution and unrelated-entity retention integration
remain later Plan 021 gates.

## Provenance

TablePro was used only to confirm the broad concepts of favorite connections
and manual/alphabetical organization. No source, tests, text, screenshots,
layouts, measurements, colors, assets, or key bindings were copied or
translated.
