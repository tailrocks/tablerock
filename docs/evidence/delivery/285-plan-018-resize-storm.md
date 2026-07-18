# Plan 018 residual — SIGWINCH / resize storm unit

Date: 2026-07-18

## What landed

- `crates/tablerock-tui/tests/shell.rs::resize_storm_last_geometry_wins_and_renders`
  - 32× multi-size Resize flood
  - every resize requests paint
  - post-storm render does not panic / empty-buffer

## Commands

```bash
cargo test -p tablerock-tui --test shell resize_storm
```

## Residual

- Scheduled CI disk-full + real PTY resize flood under load
- Perf budget publish on fixed-spec runners
