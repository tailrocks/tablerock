# 571 — Bounded bridge shutdown

Date: 2026-07-21

## Decision

The UniFFI bridge now enforces `shutdown(deadline_ms)` instead of accepting and
ignoring the deadline. During the bounded interval it drains active engine
updates, applies terminal state through the normal bridge path, retires finished
operations, and completes runtime shutdown when no operation remains.

At deadline, graceful shutdown returns the exact remaining active-operation
count and stays in draining state. Cancel-active shutdown dispatches client
stops and uses the same bounded drain. No completion is fabricated when the
deadline expires.

The engine service exposes only active operation IDs for this lifecycle
coordination; database/client types remain private.

## Verification

```text
cargo test -p tablerock-ffi --test facade
cargo test -p tablerock-engine --test engine_service
cargo clippy -p tablerock-engine -p tablerock-ffi --all-targets --locked -- -D warnings
```

Results: 9 bridge facade tests and 7 engine-service tests pass; clippy reports
no issues. A pending driver proves cancel-active shutdown reaches `Stopped`
inside its one-second bound. A separate graceful case proves a 20 ms deadline
returns `Draining { active_operations: 1 }`, then a later cancel-active call
drains cleanly.

## Remaining boundary

The bridge deadline is a monotonic in-process bound. OS termination, process
kill, and crash recovery require packaged-application lifecycle evidence.

## Provenance

Implementation source: TableRock-owned bridge runtime, engine-service, and
operation lifecycle contracts.

TablePro influence: none; this is lifecycle/safety infrastructure.

Copied source, tests, identifiers, assets, strings, colors, geometry, layout
measurements, or key bindings: none.
