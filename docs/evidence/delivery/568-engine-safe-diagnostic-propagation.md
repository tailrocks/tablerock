# 568 — Engine safe-diagnostic propagation

Date: 2026-07-21

## Decision

`AdapterError` now projects its closed engine and failure-class enums into a
conservative `SafeDiagnostic` before the engine service retires an operation.
The service retains that typed projection only until its terminal operation is
consumed or retired. The native bridge consumes it into the bounded support
bundle alongside the coarse terminal outcome.

The projection never accepts a driver or server message. Unknown operation
safety and outcome certainty remain explicitly `Unknown`; the mapper does not
invent retry safety or engine-specific error codes. Application codes are
limited to facts established by the adapter class: timeout, resource limit, or
internal failure.

This closes the architecture gap identified by evidence 567: typed failure
facts now cross the engine-service boundary instead of being reconstructed
from a terminal label.

## Verification

```text
cargo test -p tablerock-engine --lib
cargo test -p tablerock-engine --test engine_service
cargo test -p tablerock-ffi --test facade
cargo clippy -p tablerock-engine -p tablerock-ffi --all-targets --locked -- -D warnings
```

Results: 112 engine unit tests, 7 engine-service tests, and 7 bridge facade
tests pass; clippy reports no issues. The forced-query-failure bridge test
proves both `PostgreSql|Server|None|Error` and `PostgreSql|Failed` are retained
while sentinel SQL text remains absent.

## Remaining boundary

The projection is deliberately coarse because adapter failures do not yet
carry safe engine-specific codes. PostgreSQL SQLSTATE, ClickHouse numeric code,
Redis error kind, safe source position, and precise operation safety require
typed propagation from their existing lower-level diagnostic contracts. The
TUI one-shot command still has no long-lived collector. Crash-report collection
and sanitization remain unimplemented.

## Provenance

Implementation source: TableRock-owned adapter, engine-service, diagnostic,
bridge, and support-bundle contracts.

TablePro influence: none; this is diagnostics/security infrastructure, not a
product workflow or visual-expression checkpoint.

Copied source, tests, identifiers, assets, strings, colors, geometry, layout
measurements, or key bindings: none.
