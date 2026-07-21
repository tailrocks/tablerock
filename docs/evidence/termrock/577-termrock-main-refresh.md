# 577 — TermRock main compatibility refresh

Date: 2026-07-21

## Change

TableRock advances its exact TermRock 0.11.0 pin from
`dd8bed132903dbe3a8113d72940f23928716f498` to current `main` revision
`d14c6265b6f0b6f8de7c15cb18df6bbaa4edf1ec`.

The upstream comparison is nine commits, all confined to formatting and CI
maintenance. No public component contract or migration changed. TableRock
therefore needs no source migration or compatibility layer.

## Verification

```text
cargo test -p tablerock-tui -p tablerock-cli --locked
cargo clippy -p tablerock-tui -p tablerock-cli --all-targets --locked -- -D warnings
```

Results: 353 tests pass across 16 suites, 7 explicitly ignored live/process
cases remain unchanged, and clippy reports no issues. Cargo.lock resolves only
the new exact TermRock revision.

## Provenance

Implementation source: TermRock public API at the pinned revision and
TableRock-owned terminal tests.

TablePro influence: none; this is shared terminal dependency maintenance.

Copied source, tests, identifiers, assets, strings, colors, geometry, layout
measurements, or key bindings: none.
