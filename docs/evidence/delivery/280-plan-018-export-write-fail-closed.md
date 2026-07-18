# Plan 018 residual — export write fail-closed injection

Date: 2026-07-18

## What landed

- `file_effects` unit: `create_fails_closed_when_parent_is_a_file`
  - Parent path is a regular file (portable stand-in for unwritable target)
  - `AtomicFileWriter::create` returns `Err`
  - No `tablerock-tmp` debris left in the scratch directory

Complements existing atomic finish/abort/drop cleanup tests.

## Commands

```bash
cargo test -p tablerock-cli --lib file_effects
```

## Residual

- Full disk-full simulation on fixed-spec runners
- SIGWINCH / PTY storm harness in CI
