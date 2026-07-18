# Plan 018 residual — first-row / streaming budgets (local Docker)

Date: 2026-07-18
Host: macOS arm64, Docker available
Commit baseline: `ebdbeca`+

## Command

```bash
cargo test -p tablerock-engine --test performance_real current_servers_meet -- --nocapture
```

## Budgets (test constants)

| Metric | Budget |
|--------|--------|
| First page | ≤ 5 s |
| Full 10k stream | ≤ 15 s |
| Min rows/s | ≥ 500 |
| Max page resident | ≤ 2 MiB |
| Process RSS | ≤ 512 MiB |

## Observed (local rig)

| Engine | First page | Total 10k | Rows/s | Max page B |
|--------|------------|-----------|--------|------------|
| PostgreSQL 18.4-alpine | 8.33 ms | 20.9 ms | ~477k | 14_642 |
| ClickHouse 26.3 | 4.53 ms | 9.7 ms | ~1.03M | 16_643 |
| Redis 8.8.0 | 2.23 ms | 38.3 ms | ~261k | 21_149 |

Process RSS after suite: **28_917_760** bytes (~27.6 MiB).

All engines **PASS** budgets by large margins on this host.

## Status

| Claim | State |
|-------|--------|
| Local first-row/stream budgets | recorded (this doc) |
| Fixed-spec CI runners | still residual (host-specific numbers) |

## Transcript

Captured under implementer scratch `perf_real3.log` for this session.
