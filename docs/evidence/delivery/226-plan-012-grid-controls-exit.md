# Plan 012 grid controls exit

Date: 2026-07-18

## Checkpoint

Plan 012 closed for the security-critical and model/action surface:

| Step | Evidence |
|------|----------|
| BrowsePlan builder | 223 |
| Sort state + copy formats + OSC 52 | 224 |
| Filter rebrowse + column layout | 225 |
| Six copy actions wired | this doc |

## Residual (explicit, non-blocking for plan index DONE)

- Visual filter chip strip / raw-WHERE input field polish
- VirtualGrid header click hit regions for sort (CycleSort action works)
- Column drag reorder / live resize handles
- Docker sort+filter E2E fixture (builder + unit tests cover construction)

## Verification

- `cargo test -p tablerock-engine --lib browse_plan`
- `cargo test -p tablerock-tui -p tablerock-cli --lib`
- `cargo test -p tablerock-persistence --test column_layout`
- No naive `split(';')` in editor paths; values only as `$n` in browse_plan
