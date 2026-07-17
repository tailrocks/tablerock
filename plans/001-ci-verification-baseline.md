# Plan 001: Add a build/test/lint CI workflow so main is continuously verified

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat d8b113b..HEAD -- .github/ Cargo.toml`
> If these paths changed since this plan was written, compare the "Current
> state" section against the live tree before proceeding; on a mismatch, STOP.

## Status

- **Priority**: P1
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: dx
- **Planned at**: commit `d8b113b`, 2026-07-18

## Why this matters

The repository's only CI workflow is `.github/workflows/dependencies.yml`
(dependency freshness + `cargo-deny`). Nothing on GitHub builds the workspace,
runs tests, or runs clippy. The project's delivery model is trunk-only,
forward-only commits pushed directly to `main` (`AGENTS.md`), which makes an
automated build/test gate the *only* systematic safety net — there are no PRs
to review. Every later plan in `plans/` names verification commands; this plan
makes them run on every push.

## Current state

- `.github/workflows/dependencies.yml` — the only workflow. Runs on push to
  main + daily cron: `cargo outdated` freshness gate, SHA-pin staleness checks
  via `gh api`, and `EmbarkStudios/cargo-deny-action`. Uses `runs-on: macos-15`
  and SHA-pinned actions (e.g. `actions/checkout@9c091bb2… # v7.0.0`).
- Workspace: 5 crates (`crates/tablerock-{cli,core,engine,persistence,tui}`),
  resolver 3, `rust-version = "1.97"`, edition 2024 (`Cargo.toml:1-10`).
- Workspace lints already deny `clippy::correctness/suspicious/perf` and forbid
  `unsafe_code` (`Cargo.toml:32-39`) — clippy in CI enforces them.
- Tests split into two classes:
  - **Container-free**: all tests in `tablerock-core`, `tablerock-persistence`,
    `tablerock-tui`, `tablerock-cli` (the cli PTY tests spawn the real binary
    in a portable PTY — no Docker), plus `tablerock-engine` unit tests
    (`cargo test -p tablerock-engine --lib`) and its container-free
    integration targets.
  - **Docker-required**: `crates/tablerock-engine/tests/{postgres_real,clickhouse_real,redis_real,performance_real,three_engine_overlap_real}.rs`
    use `testcontainers` (see `crates/tablerock-engine/Cargo.toml:25`) and are
    NOT gated by `#[ignore]` — plain `cargo test --workspace` tries to start
    real servers.
- Repo conventions: every CI action pinned to a full commit SHA with a
  version comment, and `dependencies.yml` cross-checks those pins against the
  latest release via `gh api`. New actions must follow both conventions.
- AGENTS.md: adopt tools at latest stable; exact pins protect reproducibility.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Build | `cargo check --workspace --all-targets` | exit 0 (verified 2026-07-18) |
| Container-free tests | `cargo test -p tablerock-core -p tablerock-persistence -p tablerock-tui -p tablerock-cli` | all pass |
| Engine unit tests | `cargo test -p tablerock-engine --lib` | all pass |
| Engine real-server tests | `cargo test -p tablerock-engine --test postgres_real` (needs Docker) | all pass |
| Lint | `cargo clippy --workspace --all-targets` | exit 0 |
| Format | `cargo fmt --all --check` | exit 0 |
| Workflow syntax | `gh workflow list` after push | new workflow listed |

## Scope

**In scope** (the only files you should create/modify):
- `.github/workflows/checks.yml` (create)
- `.github/workflows/dependencies.yml` (extend the stale-pin check for any
  newly introduced action)
- `docs/evidence/` — one new numbered evidence doc + one line in
  `docs/evidence/README.md` (repo rule: every checkpoint records evidence)

**Out of scope**:
- Changing any Rust source or test.
- Adding `#[ignore]` attributes or feature-gating tests (behavior change —
  belongs to the owning crate's plans).
- Release/packaging workflows (Phase 11/12 plans own those).
- Coverage tooling, caching optimizations beyond `Swatinem/rust-cache` (skip
  caching entirely if you prefer fewer pinned actions).

## Git workflow

Repo is trunk-only (`AGENTS.md`): work directly on `main`, never branch, never
open a PR. One focused commit, Conventional Commit subject (e.g.
`ci: add build, test, and lint workflow`), DCO sign-off (`git commit -s`),
push immediately.

## Steps

### Step 1: Author `.github/workflows/checks.yml`

Two jobs, triggered on `push` to `main` and `workflow_dispatch`:

1. `checks` on `ubuntu-latest` **and** `macos-15` (matrix): checkout,
   `dtolnay/rust-toolchain@stable` (reuse the exact SHA pin already in
   `dependencies.yml`), then run in order:
   - `cargo fmt --all --check`
   - `cargo clippy --workspace --all-targets`
   - `cargo check --workspace --all-targets`
   - `cargo test -p tablerock-core -p tablerock-persistence -p tablerock-tui -p tablerock-cli`
   - `cargo test -p tablerock-engine --lib`
2. `real-servers` on `ubuntu-latest` only (Docker preinstalled): run
   `cargo test -p tablerock-engine --test postgres_real --test clickhouse_real --test redis_real --test three_engine_overlap_real`.
   Give this job `timeout-minutes: 45`. Do NOT include `performance_real` in
   CI yet — budget guardrails on shared runners flake; note that exclusion in
   the evidence doc as a visible gap.

Pin every action to a full commit SHA with a `# vX.Y.Z` comment, matching the
style in `dependencies.yml`. `permissions: contents: read`.

**Verify**: `cargo fmt --all --check && cargo clippy --workspace --all-targets && cargo test -p tablerock-core -p tablerock-persistence -p tablerock-tui -p tablerock-cli && cargo test -p tablerock-engine --lib` → all exit 0 locally.

### Step 2: Extend the stale-pin audit

If Step 1 introduced any action not already pinned in `dependencies.yml`
(e.g. a cache action), add matching `gh api` freshness assertions to the
"Reject stale CI action pins" step, following the existing `test "$(gh api …)"`
pattern. If you reused only existing pins, skip this step.

**Verify**: `grep -c 'gh api' .github/workflows/dependencies.yml` covers every
distinct action SHA used across both workflows.

### Step 3: Evidence doc + index line

Create the next-numbered evidence doc (check the highest number in
`docs/evidence/README.md`; frontier was 191 at planning time) under an
appropriate group, recording: decision (CI gate), exact jobs, what is excluded
(`performance_real`), and verification. Add its one-line entry to the evidence
index. Do not touch `ROADMAP.md`.

**Verify**: `ls docs/evidence/**/*ci*` shows the new doc; index references it.

### Step 4: Commit, push, confirm green

Commit (`git commit -s`), push to `main`, then watch the run:
`gh run watch` or `gh run list --workflow=checks.yml --limit 1`.

**Verify**: latest `checks.yml` run concludes `success`.

## Test plan

No new Rust tests. The workflow itself is the test: both jobs green on the
push that introduces it.

## Done criteria

- [ ] `.github/workflows/checks.yml` exists; every action SHA-pinned with a version comment
- [ ] Local: fmt, clippy, check, container-free tests all exit 0
- [ ] `gh run list --workflow=checks.yml --limit 1` shows `success` on `main`
- [ ] Evidence doc added + indexed
- [ ] No files outside the in-scope list modified (`git status` clean)
- [ ] `plans/README.md` status row updated

## STOP conditions

- Any container-free test fails locally at `d8b113b`-descended HEAD before
  your change — the baseline is broken; report, do not "fix" tests.
- The `real-servers` job fails in CI for infrastructure reasons (image pulls,
  Docker limits) twice in a row — report with the run URL; do not weaken or
  skip tests to force green.
- You find yourself editing any `*.rs` file.

## Maintenance notes

- `performance_real` stays local-only until a dedicated performance runner
  decision; revisit at the Phase 11 plan (018).
- When plan 002 adds new engine integration test files, add them to the
  `real-servers` job's `--test` list in the same commit.
- The dependencies workflow will start failing when the pinned actions
  release new versions — that is by design (freshness gate); refresh pins
  forward.
