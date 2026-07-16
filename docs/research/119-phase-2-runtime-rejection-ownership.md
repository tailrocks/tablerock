# Phase 2 runtime rejection ownership

Date: 2026-07-17

`DriverRuntime::spawn` is asynchronous because it accepts ownership of a driver
session. Duplicate-operation and capacity rejection now consume the rejected
session through `DriverSession::shutdown`; no rejection path silently drops an
owned connection or driven PostgreSQL task.

`DriverSpawnError` separately preserves the primary bounded runtime reason and
an optional redacted adapter shutdown failure. This avoids replacing capacity
or identity evidence while still exposing failed cleanup. It contains no raw
driver error, endpoint, query, credential, or cell value.

Contract tests prove successful cleanup for duplicate and capacity rejection,
plus safe propagation of a simulated cleanup failure. The full real-server
Testcontainers and workspace gates remain green.

This checkpoint uses only TableRock-owned architecture, tests, and existing
adapter contracts. No external-product source or protected expression was used.
