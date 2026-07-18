# Plan 021 — persistent native profile groups

Date: 2026-07-19

## Structural correction

Groups previously existed only as a nullable profile column. An empty named
group could not exist, so “create group” depended on assigning its first
profile. Migration `0014-profile-groups.sql` adds a strict registry and
backfills every distinct pre-existing group. Profile create/replace registers
assigned groups atomically. Empty groups now survive relaunch.

Create, rename, delete, and list run through the serialized local Turso actor.
Rename updates the registry and every member in one transaction. Delete moves
members to ungrouped and removes only the registry row. Both batch operations
advance every affected aggregate revision atomically; a concurrently open
editor therefore fails its compare-and-swap instead of silently restoring a
stale group.

UniFFI validates bounded group names using the core `ProfileGroupName` contract
before invoking persistence. Swift owns only dialog/collapse presentation.

## Native behavior

The sidebar renders registered groups, including empty groups, as collapsible
section headers with member counts. **New group**, **Rename Group**, and
**Remove Group** are explicit controls. Removal confirmation states that
connections move to Ungrouped and none are deleted. Search hides unrelated
empty branches while preserving every matching profile's group structure.

## Evidence

| Gate | Result |
|---|---|
| migration backfill from schema 13 | pass |
| empty group create/relaunch/rename/delete | pass |
| rename/delete member revision bump + ungroup retention | pass |
| persistence suite | pass |
| UniFFI conformance | pass |
| strict Swift 6 build | pass |
| native group structural/runtime gate | pass; two empty sections projected into live hosting tree |
| editor and AppKit accessibility runtime gates | pass |

Manual/alphabetical ordering controls remain a later Plan 021 profile-
organization checkpoint.

## Provenance

TablePro was used only to confirm the broad concept of named, collapsible
connection groups. No source, tests, text, screenshots, layouts, measurements,
colors, assets, or key bindings were copied or translated.
