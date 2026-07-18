# Plan 021 — native connection organization and search

Date: 2026-07-19

## Contract

The shared bounded profile-list projection now includes the optional default
database/logical-database context beside host and port. Persistence decodes all
three target parts once; profile search uses that same projection for host and
context matching. UniFFI exposes opaque profile IDs plus presentation facts,
not database-client types or credentials. Swift delegates search to Rust and
rejects stale async completions by generation.

## Native behavior

The connection sidebar preserves Rust ordering within named SwiftUI `List`
sections and keeps ungrouped profiles explicit. Search filters by name, host,
database, group, and existing Rust-supported tags. Rows show engine,
`host:port/database`, environment label, safety mode, favorite, connection
state, production warning with label plus symbol, and acknowledged plaintext
warning. Loading, persistence failure, empty store, and no-match states are
explicit.

Visible connection actions, collapsible/group mutation controls, editor, and a
native interaction fixture remain later Plan 021 checkpoints; this evidence
does not claim the complete connection screen.

## Evidence

| Gate | Result |
|---|---|
| shared profile-list projection tests | pass |
| persistence profile-store tests | 35 passed |
| UniFFI conformance, including case-insensitive database search and no-match | 17 passed, 5 real-server tests ignored |
| strict native build and regenerated bindings | pass |
| native structural/runtime accessibility gate | pass |

## Provenance

TablePro was used only to confirm the broad concept of a grouped, searchable
database connection list. No source, tests, text, screenshots, layouts,
measurements, colors, assets, or key bindings were copied or translated.
