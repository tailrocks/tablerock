# Plan 021 — durable history retention

Date: 2026-07-19

## Persistence

Migration `0016-history-retention.sql` adds one strict singleton preference
with encoded values Full, Metadata only, or Private. Existing stores backfill
Full, matching prior behavior. `CREATE TABLE IF NOT EXISTS` plus
`INSERT OR IGNORE` keeps forward repair deterministic when a migration marker
is missing but the exact table already exists.

Get/set commands run only through the serialized local Turso actor. Invalid or
missing singleton values fail closed. Relaunch tests change Full to Metadata
only, reopen the actor, verify the value, then change and read Private.

## Bridge and native behavior

Persistence configuration reads retention before installing the actor into the
bridge. Terminal history capture therefore cannot race ahead using a stale
default. Retention updates persist first and mutate bridge memory only after
actor success. Native initialization reads the durable value before presenting
Query History, so the picker and subsequent operations match after relaunch.

## Evidence

| Gate | Result |
|---|---|
| migration 16 fresh-store default | pass; Full |
| schema 15 forward migration | pass |
| actor set/reopen Metadata only and Private | pass |
| bridge load/set projection | pass |
| persistence suite | pass; 38 tests |
| UniFFI conformance | pass; 18 tests, 5 ignored |
| TUI suite | pass; 315 tests |
| CLI suite | pass; 53 tests, 7 ignored |
| native history structural/runtime gate | pass |
| native profile/editor/accessibility regression gates | pass |

## Provenance

TablePro was used only to confirm the broad concept of configurable local query
history retention. No source, tests, text, screenshots, layouts, measurements,
colors, assets, or key bindings were copied or translated.
