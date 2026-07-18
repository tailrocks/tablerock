# Plan 016 residual — multi-named filter presets

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| SaveFilter opens name confirm (safe charset) | done |
| ApplyFilter opens name confirm + known-name hints | done |
| `is_safe_preset_name` / `names_for_table` | done |
| Persist still via library JSON actor (306) | done |
| Unit: named round-trip + charset | done |

## Decision

Fixed `default` name was enough for first wiring. Operators now paste a
named preset (`[A-Za-z0-9._-]{1,64}`) on save and apply. Known names for the
active table are shown on apply. Hostile names fail closed with no I/O.

## Evidence

```text
cargo test -p tablerock-tui --lib filter
```

## Remaining work

- ~~Fuzzy picker UI for many presets~~ (closed: evidence 319)
