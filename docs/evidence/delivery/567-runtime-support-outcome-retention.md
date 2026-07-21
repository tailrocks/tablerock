# 567 — Runtime support-outcome retention

Date: 2026-07-21

## Decision

The native bridge retains non-ordinary engine terminal outcomes below the
presentation layer. Each retained record contains only two closed Rust enums:
database engine and `OperationOutcome`. Driver errors, server messages, SQL,
cell values, endpoints, paths, and UI failure labels cannot enter this path.

The bridge owns the bounded runtime-instance bundle beside its runtime state and
exports that retained state atomically. Completed operations are omitted;
failed, disconnected, cancelled, stopped, race-completed, and unknown terminal
outcomes are retained. Saturation is explicit. The additive record section
advances the support format from schema 1 to schema 2.

This is a structural fix for the unsafe alternative: TUI `FailureProjection`
labels may contain arbitrary strings, so support export never scrapes them.

## Verification

```text
cargo test -p tablerock-core --test support_bundle
cargo test -p tablerock-cli --test process_contract
cargo test -p tablerock-ffi --test facade
cargo clippy -p tablerock-core -p tablerock-cli -p tablerock-ffi --all-targets --locked -- -D warnings
```

Results: 4 core support tests, 4 CLI process tests, and 7 bridge facade tests
pass; clippy reports no issues. The bridge failure test submits a statement
containing sentinel secret text, forces a driver failure, exports the bundle,
and proves only `PostgreSql|Failed` appears.

## Remaining boundary

Coarse runtime outcomes are retained; engine-specific `SafeDiagnostic` facts
are not yet propagated through the engine-service terminal event. The TUI
one-shot command has no long-lived process collector. Crash-report collection
and sanitization remain unimplemented. Explicit bridge runtime destruction
clears retained outcomes.

## Provenance

Implementation source: TableRock-owned operation lifecycle, safe-diagnostic,
bridge, and atomic-file contracts.

TablePro influence: none; this is diagnostics/security infrastructure, not a
product workflow or visual-expression checkpoint.

Copied source, tests, identifiers, assets, strings, colors, geometry, layout
measurements, or key bindings: none.
