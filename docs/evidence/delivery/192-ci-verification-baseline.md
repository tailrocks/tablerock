# CI verification baseline

Date: 2026-07-18

## Checkpoint

Plan 001. GitHub Actions now builds, lints, and tests the workspace on every
push to `main`. Dependency freshness remains in `dependencies.yml`; this
checkpoint adds the missing compile/test safety net for trunk-only delivery.

## Decision

One workflow, `.github/workflows/checks.yml`, with two jobs:

| Job | Runner | Purpose |
|---|---|---|
| `checks` | matrix: `ubuntu-latest`, `macos-15` | `fmt`, `clippy`, `check`, container-free tests, engine `--lib` |
| `real-servers` | `ubuntu-latest` only, 45-minute timeout | Docker-backed engine integration tests |

Container-free packages: `tablerock-core`, `tablerock-persistence`,
`tablerock-tui`, `tablerock-cli`, plus `tablerock-engine --lib`.

Real-server targets: `postgres_real`, `clickhouse_real`, `redis_real`,
`three_engine_overlap_real`.

Action pins reuse the exact SHAs already audited in `dependencies.yml`
(`actions/checkout@v7.0.0`, `dtolnay/rust-toolchain@stable`). No new action
vendor was introduced, so the stale-pin step did not need extension.
`permissions: contents: read`. Triggers: `push` to `main` and
`workflow_dispatch`.

## Bounds and failure truth

- `performance_real` is **excluded** from CI. Shared-runner timing flakes; the
  budget suite stays local until Phase 11 (plan 018) decides a dedicated runner.
- Real-server tests remain ungated by `#[ignore]`; CI simply does not select
  them in the container-free job and runs them only where Docker is available.
- No Rust source or test files were modified.
- Workflow failure blocks nothing in git (trunk-only, no required status
  checks configured here); green runs are the operator and agent verification
  signal after each push.

## Evidence

- Local gate on this machine before push:
  `cargo fmt --all --check`
  `cargo clippy --workspace --all-targets`
  `cargo check --workspace --all-targets`
  `cargo test -p tablerock-core -p tablerock-persistence -p tablerock-tui -p tablerock-cli`
  `cargo test -p tablerock-engine --lib`
- GitHub run for the introducing commit must conclude `success` (both jobs).

## Remaining work

- Add new engine integration targets to the `real-servers` job when plan 002
  (and later) introduce them.
- Revisit `performance_real` at plan 018.
- Optional: cache (`Swatinem/rust-cache`) if wall-clock cost becomes a problem;
  skipped here to avoid an extra pinned action.
