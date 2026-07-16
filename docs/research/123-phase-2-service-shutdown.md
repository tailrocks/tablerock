# Phase 2 service shutdown coordination

Date: 2026-07-17

`EngineService` now coordinates core shutdown state with runtime task ownership.
`begin_shutdown(Graceful)` stops new submission and lets active tasks reach their
observed terminal outcomes. `begin_shutdown(CancelActive)` first moves every
active core record to `CancelRequested`, then sends a latest-state client-stop
signal to each matching runtime task.

Client stop is intentionally distinct from server cancellation. It guarantees
local task drain without claiming a cancellation request reached the server.
The bounded shutdown result returns one operation ID plus
`RuntimeStopOutcome` per requested client stop. A joined `ClientStopped` task
exit maps to the legal core `ClientStopped` outcome only from
`CancelRequested`.

Terminal event delivery may be full during forced stop. The joined task exit is
authoritative and reconstructs the terminal core outcome when the event channel
closes; task panic/join failure still becomes `Unknown`. This prevents slow
presentation from blocking shutdown while preserving outcome truth.

`complete_shutdown` fails while core/runtime work is still draining. Once core
is `Stopped` and every runtime receiver is reconciled, it consumes runtime
shutdown exactly once. Tests prove graceful completion, cancel-active client
stop, premature completion rejection, terminal reconstruction, session
shutdown, and final runtime release.

This checkpoint uses TableRock-owned contracts and direct tests only. No
external-product source or protected expression influenced it.
