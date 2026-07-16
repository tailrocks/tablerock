# Phase 2 operation-to-driver routing

Date: 2026-07-17

`DriverOperationRegistry` is the first engine-owned operation routing seam. It
maps bounded core `OperationId` values to type-erased driver sessions, rejects
duplicate identity and capacity overflow, and forwards cancellation only to the
registered session.

The registry deliberately does not own lifecycle truth. Unknown operations are
reported as unknown, unsupported adapters remain unsupported, and request
delivery is not reported as server-confirmed cancellation. Callers remove a
session only after the core coordinator observes a terminal operation state;
the consuming session shutdown remains explicit.

Contract tests prove capacity, duplicate identity, unknown cancellation,
unsupported cancellation, removal, and consuming shutdown without contacting a
server. Real Testcontainers suites continue to prove each driver boundary.

This checkpoint introduces no external-product influence. Sources are the
approved TableRock architecture and shared-client contract.
