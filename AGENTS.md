# AGENTS.md

Primary branch: `main`.

## Current phase

TableRock is research-only. Do not add application code or dependencies until
the relevant research decision and roadmap phase are approved.

## Product boundary

- TableRock owns PostgreSQL, ClickHouse, and Redis connection, exploration,
  query, result, edit, history, and safety behavior.
- The first UI is a Rust CLI/TUI.
- A future native macOS UI uses SwiftUI/AppKit over the Rust core.
- Shared terminal components come from the independent Tailrocks TUI project;
  TableRock does not import `jackin` product internals.

## Clean-room rule

TablePro, TablePlus, and Zedis may establish that a broad problem or workflow
exists. Never copy or translate their source, tests, comments, identifiers,
assets, product text, screenshots, layout measurements, colors, or key bindings.
Implement from this repository's requirements, official database documentation,
selected library documentation, and direct tests.

Record external-reference provenance in every implementation PR.

## Engineering

- Prefer maintained crates and official clients over hand-written protocols.
- The ClickHouse baseline is the official `ClickHouse/clickhouse-rs` client.
- The Redis baseline is `redis-rs/redis-rs`; do not substitute another client
  without an explicit architecture decision.
- Keep database client types behind adapters and out of stable core contracts.
- Batch/page results across UI, daemon, and future FFI boundaries.
- Keep I/O out of TUI update/render functions.
- Enforce read/write safety and redaction below presentation.
- Never persist resolved 1Password values or log credentials, SQL text, or cell
  values by default.
- Update research, roadmap, user documentation, and tests with behavioral
  changes.

## Commits

Use Conventional Commits, DCO sign-off (`git commit -s`), and push each commit
immediately unless the operator explicitly says otherwise.
