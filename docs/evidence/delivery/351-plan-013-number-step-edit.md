# Plan 013 residual — number step Inc/Dec while editing

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `CellEditSession::step_number` for Number kind | done |
| Integer + finite float step; null/empty → delta | done |
| `ActionId::IncNumber` / `DecNumber` toolbar | done |
| Non-number kinds no-op | done |
| Unit test | done |

## Decision

Type-specific number widgets remain optional; Inc/Dec give keyboard-friendly
staging without a spinner control. Engine re-types on apply.

## Evidence

```text
cargo test -p tablerock-tui --lib number_step
cargo check -p tablerock-tui
```

## Remaining work

- Enum picker / JSON tree editor widgets (optional)
