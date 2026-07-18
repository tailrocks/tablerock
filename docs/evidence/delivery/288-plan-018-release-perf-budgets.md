# Plan 018 residual — release-profile performance budgets (local rig)

Date: 2026-07-18

## Environment

| Fact | Value |
|------|-------|
| Host | macOS arm64 (Apple Silicon) |
| Binary | `target/release/tablerock-cli` |
| Size | 31_164_016 bytes (~29.7 MiB) |
| File | Mach-O 64-bit executable arm64 |

## Cold process start (no TTY)

`tablerock-cli --help` without an interactive terminal exits immediately with
`TableRock: interactive terminal required` after argument parse.

| Run | real (s) |
|-----|----------|
| 1 | 0.02 |
| 2 | 0.01 |
| 3 | 0.02 |
| 4 | 0.01 |
| 5 | 0.02 |

| Stat | Value |
|------|-------|
| min | 0.010 s |
| median | 0.020 s |
| max | 0.020 s |

### Budget (local rig, process-start only)

| Metric | Budget | Observed | Status |
|--------|--------|----------|--------|
| Cold process start (no TTY, --help) | ≤ 100 ms | ≤ 20 ms | PASS |
| Release binary size | ≤ 64 MiB | ~29.7 MiB | PASS |

## Not measured here (still residual)

- Full TUI enter + first paint (requires interactive PTY harness timings)
- First-row query latency against Docker engines
- Resident-scroll FPS under VirtualGrid million-row synthetic
- Fixed-spec CI runners (numbers above are **local rig only**)

## Commands

```bash
cargo build -p tablerock-cli --release
/usr/bin/time -p ./target/release/tablerock-cli --help
```
