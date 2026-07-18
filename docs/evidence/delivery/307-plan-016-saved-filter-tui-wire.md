# Plan 016 residual — saved filter TUI load/save wiring

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| Workbench `filter_library` field | done |
| Connect path: intent → load library → catalog | done |
| `SaveFilter` upserts `default` preset + persist effect | done |
| `ApplyFilter` restores preset and re-browses | done |
| CLI effect handlers for put/get library | done |
| Unit tests for save/apply and load→catalog | done |

## Decision

Named presets live in the workbench library. Connect for non-temporary
profiles loads the profile library after session intent (fail-open: load
failure still opens catalog). Operators save the active grid filters as the
`default` preset for `schema.table` and re-apply with `LoadFilt`. JSON never
carries cell payloads or credentials.

## Evidence

```text
cargo test -p tablerock-tui filter
cargo test -p tablerock-persistence --test saved_filter_library
cargo test -p tablerock-cli --lib
```

## Remaining work

- Multi-named preset picker UI (beyond fixed `default` name).
