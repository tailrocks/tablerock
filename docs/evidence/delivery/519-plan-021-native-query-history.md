# Plan 021 — native query history

Date: 2026-07-19

## Rust ownership

Native query submission now captures a bounded statement and saved/default
database context only for `execute` intent. Terminal engine outcome maps to the
existing persistence history taxonomy and appends through the serialized local
Turso actor. Probe, catalog, and page-fetch operations never become history.
Result payloads never persist.

History failure emits a separate safe `history_failed` event. Query completion
and result truth remain unchanged; native presentation shows the local-storage
warning alongside the successful result.

The bridge exposes newest-first search with a 1–500 row bound and a 256-byte
search bound. Records contain safe engine/context/outcome/time facts and
optional statement text. Full, metadata-only, and private retention affect
subsequent operations; metadata rows keep no SQL, private mode appends no row.

## Native screen

**Query History** has explicit loading, empty, no-match, and failure states.
Search matches retained SQL text. Rows distinguish omitted SQL from empty data,
show engine/context/outcome/time, and restore retained text into the current
editor without execution. Metadata-only rows are visibly unavailable. A
retention picker exposes Full SQL, Metadata only, and Private.

History rows are durable. Schema-16 durable retention and relaunch projection
landed in [evidence 520](520-plan-021-durable-history-retention.md).

## Lifecycle corrections

Conformance exposed two earlier bridge defects. Multiple sessions for one
saved profile attempted to re-register the shared profile scope, so successful
reconnect was impossible. Profile scope is now shared until its last session
disconnects. Terminal operations also remained resident after bridge pumping,
blocking context/session cleanup. Pump now retires each terminal operation;
disconnect removes context, session, and last-owner profile scopes plus cached
catalog nodes.

## Evidence

| Gate | Result |
|---|---|
| terminal execute records full SQL and completed outcome | pass |
| metadata-only omits SQL; private appends nothing | pass |
| bounded search and restore-without-execute | pass |
| same-profile multi-session + scope cleanup | pass |
| core suite | pass; 145 tests |
| UniFFI conformance | pass; 18 tests, 5 ignored |
| TUI suite | pass; 315 tests |
| CLI suite | pass; 53 tests, 7 ignored |
| native history structural/runtime gate | pass |
| native profile/editor/accessibility regression gates | pass |

## Provenance

TablePro was used only to confirm the broad concept of searchable local query
history that restores SQL without running it. No source, tests, text,
screenshots, layouts, measurements, colors, assets, or key bindings were copied
or translated.
