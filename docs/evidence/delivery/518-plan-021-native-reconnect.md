# Plan 021 — shared bounded native reconnect

Date: 2026-07-19

## Structural correction

Reconnect timing previously lived in the TUI crate. That allowed native and
terminal retry semantics to drift. `tablerock-core` now owns the decision:
manual, authentication stop, retry-after, or exhausted. Both clients consume
that authority. Automatic reconnect attempts immediately, then waits 1, 2, 4,
8, 16, and 30 seconds. Seven attempts incur at most 61 seconds delayed.

The bridge resolves reconnect preference and context-restoration intent from
the durable profile attached to the opaque session. Temporary or deleted
profiles cannot silently become automatic saved-profile reconnects.

## Replacement safety

`reconnect_saved_session` opens the replacement under the same durable
`ProfileId` before retiring the old session. Failed open leaves the old session
registered and current native catalog, query, and result presentation intact.
If old-session retirement fails because work remains active, the new session is
closed and the reconnect fails closed. No operation is replayed, so ambiguous
writes are never retried.

Replacement attempts return structured `connected`, `retryable`, or
`authentication_stopped` state. A saved Prompt-on-connect source requires an
explicit transient override; missing credentials never degrade into an empty
password connection attempt or a retry loop. Native Connect, Test, and manual
Reconnect use one secure transient sheet and clear its field before awaiting
the bridge. Automatic reconnect stops and waits for operator authentication.

Native state exposes manual **Reconnect** plus automatic state for bounded
profiles. Disconnect or another successful connection invalidates an in-flight
schedule by generation. Authentication stops immediately; exhaustion remains
visible. Successful replacement retains workbench context and re-probes health.

## Evidence

| Gate | Result |
|---|---|
| shared core immediate/capped/exhausted policy | pass |
| manual and authentication stop | pass |
| durable preference and restore-context projection | pass |
| failed replacement preserves old connected session | pass |
| prompt source requires override + stops automatic retry | pass |
| core suite | pass; 145 tests |
| UniFFI conformance | pass; 18 tests, 5 ignored |
| TUI suite | pass; 315 tests |
| native reconnect projection runtime fixture | pass |
| native prompt Connect/Test/Reconnect structural gate | pass |

Real-server native reconnect behavior across PostgreSQL, ClickHouse, Redis,
TLS/authentication loss, and context changes remains in the Plan 021 matrix.

## Provenance

TablePro was used only to confirm the broad concept of reconnecting a database
session while preserving operator work. No source, tests, text, screenshots,
layouts, measurements, colors, assets, or key bindings were copied or
translated.
