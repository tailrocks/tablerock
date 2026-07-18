# Plan 016 residual — saved filter library persistence actor

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| Migration 0013 `saved_filter_libraries` | done |
| Actor put/get/delete for library JSON | done |
| Fail-closed validation (array-only, no cells/secrets) | done |
| Schema version 13 | done |
| Integration test | done |
| TUI load-on-connect / save-on-upsert wiring | residual |

## Decision

Named filter presets already serialize as a session-local JSON library
(`SavedFilterLibrary::to_json`). Persist one library blob per profile through
the existing single-threaded Turso actor — same pattern as column layouts —
never cell values or credentials. Hostile or non-array JSON fails closed.

## Bounds and failure truth

- `library_json` length 2..=65536, must start with `[`.
- Reject payloads containing `"cells"`, `"result`, `password`, or `secret`.
- One row per `profile_id` (upsert replaces).

## Evidence

```text
cargo test -p tablerock-persistence --lib validate_rejects
cargo test -p tablerock-persistence --test saved_filter_library
cargo test -p tablerock-persistence --test actor
```

## Remaining work

- TUI Effect load on ConnectOk and save after preset upsert.
