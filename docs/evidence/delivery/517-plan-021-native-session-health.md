# Plan 021 — native live session health

Date: 2026-07-19

## Contract

`check_session_health` borrows an opaque live session and invokes the shared
`DriverSession::health` contract. UniFFI returns only structured state,
reachability, optional elapsed milliseconds, and an authentication-stop flag.
It rejects unknown handles and engine mismatch. Raw adapter/server text and
credentials never cross the bridge.

Successful driver probes project `healthy` or `unreachable`. Connection and
timeout failures remain distinct. Authentication projects
`authentication_stopped`; clients must not blindly retry it. Other safe
failure classes collapse to `unhealthy`.

## Native behavior

Saved and temporary connections run a health check after successful open.
Connected profile rows show healthy latency or explicit unhealthy,
unreachable, timeout, and authentication-stopped states. Row menus and the
workbench toolbar expose **Check Health**. Probe execution stays in the
bridge-client actor, away from `MainActor`; Swift only renders returned facts.

## Evidence

| Gate | Result |
|---|---|
| three-engine healthy state + elapsed projection | pass |
| authentication terminal-state projection | pass |
| UniFFI conformance | pass; 18 tests, 5 ignored |
| generated Swift bindings | pass |
| native group structural/runtime fixture | pass; `Healthy · 12 ms` mapping |

Bounded automatic reconnect, backoff presentation, and context restoration
remain later Plan 021 gates.

## Provenance

TablePro was used only to confirm the broad concept of visible connection
health. No source, tests, text, screenshots, layouts, measurements, colors,
assets, or key bindings were copied or translated.
