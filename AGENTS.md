# AGENTS.md

## Trunk-only workflow

- Work directly on `main` only.
- Never create, switch to, or publish another branch.
- Never open a pull request.
- Keep `main` buildable through small, forward-only checkpoint commits. Do not
  rewrite published history; repair forward.
- These rules also apply to required TermRock changes. Jackin remains a
  read-only reference.

## Current phase

Phase 0 research decisions are approved. Implement Roadmap Phases 1-15 through
their dependency-ordered, evidence-gated checkpoints. Do not add behavior or a
dependency before its relevant roadmap checkpoint is approved and its adoption
requirements are defined.

## Product boundary

- TableRock owns PostgreSQL, ClickHouse, and Redis connection, exploration,
  query, result, edit, history, and safety behavior.
- The first UI is a Rust CLI/TUI.
- The TUI uses The Elm Architecture, TermRock, Ratatui, and Crossterm.
- The native macOS UI uses SwiftUI/AppKit over embedded Rust through synchronous
  UniFFI and ships first as a direct notarized Developer ID application.
- Shared terminal components come from the independent
  [`termrock`](https://github.com/tailrocks/termrock) crate;
  TableRock does not import `jackin` product internals.

## Clean-room rule

TablePro, TablePlus, and Zedis may establish that a broad problem or workflow
exists. Never copy or translate their source, tests, comments, identifiers,
assets, product text, screenshots, layout measurements, colors, or key bindings.
Implement from this repository's requirements, official database documentation,
selected library documentation, and direct tests.

Record external-reference provenance in every influenced implementation commit
and its accompanying requirement/test documentation.

## Engineering

- Start every dependency, toolchain, CI action, and development-tool adoption
  from its latest stable release. Re-check before use and upgrade immediately
  when a newer stable release exists. Exact pins protect reproducibility, not
  legacy compatibility; refresh them forward and document any proven temporary
  upstream constraint.
- Prefer maintained crates and official clients over hand-written protocols.
- The TUI application pattern is The Elm Architecture. Do not introduce
  Component Architecture, Flux, or component-owned application state.
- Crossterm 0.29 is the only terminal backend/input; TermRock owns terminal
  lifecycle and reusable components.
- PostgreSQL uses `tokio-postgres` with rustls; SSH uses `russh`.
- The ClickHouse baseline is the official `ClickHouse/clickhouse-rs` client.
- The Redis baseline is `redis-rs/redis-rs`; do not substitute another client
  without an explicit architecture decision.
- Keep database client types behind adapters and out of stable core contracts.
- Persistence uses the local-only `turso` crate through one serialized Rust
  async persistence actor. Never add `rusqlite`, `libsql`, or Turso Cloud sync.
- Native macOS embeds Rust through synchronous UniFFI. Do not add a daemon,
  local RPC, manual C ABI, WebView, or Mac App Store path.
- Batch/page results across TUI and UniFFI boundaries.
- Keep I/O out of TUI update/render functions.
- Enforce read/write safety and redaction below presentation.
- Never persist resolved 1Password values or log credentials, SQL text, or cell
  values by default.
- Update research, roadmap, user documentation, and tests with behavioral
  changes.

## Commits

Use Conventional Commits, DCO sign-off (`git commit -s`), and push each commit
immediately unless the operator explicitly says otherwise.
