# Plan 021 — native saved queries

Date: 2026-07-19

## Contract

Rust UniFFI now exposes saved-query list, upsert, and delete operations backed
by the serialized local Turso actor. List projection is capped at 1,000 rows;
search is capped at 256 bytes and matches names or statement text
case-insensitively. Engine filters parse exact supported engine names. Names
are limited to 128 bytes and statements to 1 MiB. Existing persistence identity
remains unique by name and engine, so saving the same pair updates it.

Native SwiftUI provides Saved Queries and Save Query toolbar actions, explicit
loading/failure/empty/no-match states, engine filtering, name/SQL search,
restore into the editor without execution, and confirmed deletion. Delete copy
states that query history remains unchanged.

## Structural correction

Native regression verification exposed a bridge catalog invariant that rejected
valid PostgreSQL function leaf nodes because their child state is
`NotApplicable`. Bridge validation now rejects only `Failed` child state while
retaining expected-kind and bounds checks. Live three-engine catalog evidence
passes after correction.

## Evidence

| Gate | Result |
|---|---|
| UniFFI saved-query create/search/update/delete conformance | pass |
| native saved-query structural/runtime fixture | pass |
| native behavior against PostgreSQL, ClickHouse, and Redis | pass |
| native profile editor/group/history/accessibility regressions | pass |
| workspace test rerun of PTY timeout | pass; isolated stress test 1.95s |

## Remaining boundary

Table/object favorites and file-backed SQL open/save/reload/external-change
handling remain separate parity work. This checkpoint does not claim them.

## Provenance

TablePro was used only to confirm the broad concepts of named reusable queries,
search/filter, editor restoration, and explicit deletion. No source, tests,
text, screenshots, layouts, measurements, colors, assets, or key bindings were
copied or translated.
