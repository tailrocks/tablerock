# 575 — Complete migration documentation

Date: 2026-07-21

## Decision

Every persistence migration from 0001 through 0018 now has a matching,
indexed explanation. The new 0008–0016 notes describe the shipped SQL's
bounded storage, closed values, defaults, backfills, indexes, and privacy
boundaries without changing any applied migration.

The migration index remains the canonical map from sequence number to its SQL
explanation. Future schema changes must add both the next zero-padded SQL file
and its matching note; applied SQL remains immutable.

## Verification

The 0008–0016 notes were checked directly against their corresponding SQL
files. `git diff --check` passes. This documentation-only checkpoint changes no
runtime, schema, dependency, or generated artifact.

## Provenance

Implementation source: TableRock-owned persistence migrations and contracts.

TablePro influence: none; this is migration audit documentation.

Copied source, tests, identifiers, assets, strings, colors, geometry, layout
measurements, or key bindings: none.
