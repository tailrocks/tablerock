# Plan 012 residual — DemoteFirstFilter

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `demote_first_filter` oldest → end | done |
| Needs ≥2 chips | done |
| Action DemoFilt + rebrowse | done |
| Unit test | done |

## Decision

PromoFilt moves newest to front. Symmetric demote moves filters[0] to the
end so the oldest chip becomes least-significant without full reverse.

## Evidence

```text
cargo test -p tablerock-tui --lib remove_last_and_column
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for demote-first filter
