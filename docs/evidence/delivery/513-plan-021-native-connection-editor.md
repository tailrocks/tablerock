# Plan 021 — native connection editor and actions

Date: 2026-07-19

## Structural contract

UniFFI now carries a bounded editable profile draft with opaque ID and expected
revision. Rust validates engine, identity, host, port, context, username, TLS,
safety, group, environment, and secret source before atomically creating or
replacing the aggregate. Duplicate allocates a fresh cross-process identity;
remove uses revision compare-and-swap and never disconnects active sessions.

Profile IDs now include a process-unique nanosecond epoch component instead of
restarting at one after every launch. Native persistence moved from temporary
storage to `~/Library/Application Support/TableRock/profiles.db`; directory
creation failure is explicit.

Secret references remain Rust-owned. Environment names and 1Password compact
ID references cross the editor; resolved values do not. Stored plaintext is
reported only as a boolean and is never returned. Saving plaintext requires
explicit acknowledgement and re-entry. Prompt values enter the engine through
a new nonempty 256-KiB bounded zeroizing constructor. Saved-profile connect now
honors stored TLS policy and resolves dangerous plaintext, environment,
1Password, or prompt override instead of silently using an empty password.

## Native behavior

Every row has a visible action menu plus context menu: Connect, Edit,
Duplicate, Test, and revision-safe Remove with destructive confirmation. New
connection opens the same SwiftUI form. It contains the specified General,
Connection, Credentials, and TLS sections for PostgreSQL, ClickHouse, and
Redis. Loading/save/removal/test failures remain explicit alerts.

Test is one Rust operation: connect, describe, disconnect. The returned safe
report contains server identity/version, successful configured TLS outcome,
and elapsed milliseconds; no profile mutation occurs.

## Evidence

| Gate | Result |
|---|---|
| UniFFI profile CRUD/search/connect conformance | 17 passed, 5 real-server tests ignored |
| engine secret-resolution focused tests | 9 passed |
| persistence tests | 35 passed |
| strict Swift 6 native build | pass |
| profile editor structural/runtime gate | pass; six text fields and all five native pickers rendered in `Edit Connection` |
| AppKit accessibility structural/runtime gate | pass |

The full real TLS/auth Test matrix, Keychain editing, group mutation controls,
and unrelated-history retention integration remain later Plan 021 gates.

## Provenance

TablePro was used only to confirm the broad concepts of a database connection
editor and adjacent row actions. No source, tests, text, screenshots, layouts,
measurements, colors, assets, or key bindings were copied or translated.
