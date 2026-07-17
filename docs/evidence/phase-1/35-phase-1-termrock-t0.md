# Phase 1 TermRock T0 Evidence

## Checkpoint

TableRock pins TermRock `0.6.0` at the full Git revision
`41482e9fe9b8254f82c7803692ef9dbd7d4cf87c`. On 2026-07-16 the upstream
`main` reference resolved to the same commit. The dependency is exact, not a
branch or floating tag, and `Cargo.lock` is committed.

The minimal `tablerock-tui` consumer proves these public seams:

- one root-owned `Model` and semantic `Message`;
- synchronous `update` returning the then-current TermRock `UpdateResult`;
- the then-current TermRock `View<Model>` rendering through `runtime::drive_frame`;
- a TermRock `Panel` rendered into Ratatui `TestBackend`;
- CLI-owned, feature-enabled TermRock `Session` setup and idempotent restoration
  against an in-memory writer with every live-terminal mode disabled;
- CLI-owned Crossterm 0.29 `EventStream` availability from the same locked tuple;
- no I/O, async runtime, database type, effect executor, or terminal lifecycle
  inside the model, update, or view.

These historical interfaces were replaced by migration 0024; see
[`130-termrock-closure-runner-frame-time-migration.md`](../termrock/130-termrock-closure-runner-frame-time-migration.md).

This checkpoint does not claim an executable shell, input stream, terminal
restoration, responsive layout, engine subscription, database capability, or
the missing T1 primitives.

## Compatibility and dependency record

| Dependency | Pin | Features | License | MSRV | Motivation |
|---|---|---|---|---|---|
| TermRock | Git `41482e9f...` / 0.6.0 | `crossterm` enabled only by CLI | Apache-2.0 | 1.95 | sole reusable TUI/runtime/session layer |
| `ratatui-core` | 0.1.2 | defaults | MIT | inherited compatibility gate | public frame, rectangle, and test backend types required by TermRock's contract |
| Crossterm | 0.29.0 | default plus `event-stream` | MIT | inherited compatibility gate | sole terminal input/backend line; EventStream contract |

The verified upstream tuple is Rust 1.95 minimum (1.97 tested),
`ratatui-core` 0.1.2, `ratatui-widgets` 0.3.2, optional
`ratatui-crossterm` 0.1.2, and Crossterm 0.29.0. T0 enables TermRock's
`crossterm` feature and Crossterm `event-stream` only in `tablerock-cli`;
`tablerock-tui` has no direct Crossterm dependency. The executable entry point
and live terminal lifecycle remain later checkpoints.

## T0 inspection

The pinned upstream component inventory supplies the existing neutral widgets
and runtime/session contracts. It does not supply `Form`, `Tree`, or
`SplitPane`; those remain TermRock T1 prerequisites. No duplicate TableRock
primitive was introduced.

Context7 was requested for current TermRock setup documentation but returned a
quota-exhausted response. The fallback evidence is the authoritative upstream
repository at the exact pinned commit: workspace manifests, README,
`compatibility.toml`, component inventory, public runtime source, scoped-session
example, and tests.

## Verification

- Red: the public consumer integration test failed because `tablerock-tui` did
  not yet expose `Model`, `Message`, `update`, or `ShellView`.
- Green: `cargo test -p tablerock-tui --test termrock_consumer`.
- Workspace: `cargo test --workspace --locked`.
- Formatting: `cargo fmt --all -- --check`.
- Lints: `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- Docs: `cargo doc --workspace --locked --no-deps` with warnings denied.
- Dependency review: duplicate/version and feature trees inspected; no
  competing TUI, terminal, persistence, database, parser, or bridge stack.
- Security/licenses: `cargo deny check advisories bans licenses sources` with
  fail-closed registry/Git policy and only the exact TermRock repository
  allowlisted; advisories, bans, licenses, and sources pass. The two
  `hashbrown` versions are transitive within Ratatui core and are recorded as a
  warning, not an alternate product stack.
- Dependency source: Cargo lock/metadata resolve TermRock to the full pinned Git
  revision.

External concept: none; neutral library adoption only  
Public source: <https://github.com/tailrocks/termrock/tree/41482e9fe9b8254f82c7803692ef9dbd7d4cf87c>  
TableRock requirement: Roadmap Phase 1 / delivery-plan TermRock T0  
Implementation source: TermRock public API at the pinned revision and TableRock tests  
Copied code/assets/text: none
