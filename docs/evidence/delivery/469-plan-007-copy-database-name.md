# Plan 007 residual — CopyDatabaseName

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyDatabaseName from context bar | done |
| Empty database no-op | done |
| Action CopyDb | done |
| Unit test (extends session/engine) | done |

## Decision

Complements CopySid/CopyEng with the active logical database string for
connection tickets and `USE`/`\\c` paste.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_session_id_and_engine
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for database name copy
