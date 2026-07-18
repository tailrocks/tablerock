# Phase 2 session registry and Arc runtime borrow

Date: 2026-07-18

## Checkpoint

Plan 002 step 2. Engine operations borrow shared sessions instead of consuming
and shutting them down at every terminal event.

## Decision

- `SessionRegistry` owns `Arc<SessionSlot>` keyed by `SessionId`, capacity up to
  1024.
- `SessionSlot` wraps `Box<dyn DriverSession>` behind `tokio::sync::RwLock` so
  concurrent start/cancel use read locks while exclusive disconnect takes a
  write lock and calls `shutdown` exactly once.
- `DriverRuntime::spawn` and `EngineService::submit` take `Arc<dyn DriverSession>`
  and **never** call `session.shutdown()` at operation end or spawn rejection.
- `EngineService::disconnect(session_id)` removes the slot and shuts it down
  only when `Arc::strong_count == 1` (no active operation borrows); otherwise
  returns `EngineServiceError::SessionBusy`.
- Core submission rejection drops the Arc borrow only.

## Bounds and failure truth

| Case | Behavior |
|---|---|
| Duplicate register | `SessionRegistryError::DuplicateSession` |
| Capacity exceeded | `SessionRegistryError::CapacityExceeded` |
| Disconnect while ops hold Arc | `SessionBusy`; session stays registered |
| Disconnect exclusive | `shutdown` once; unknown on second call |
| Closed slot start | `AdapterFailureClass::Connection` |
| Closed slot cancel | `CancelDispatch::PreventedBeforeDispatch` |

Cancel/client-stop/terminal event sequencing is unchanged; only ownership of
connection teardown moved to the registry.

## Evidence

- `cargo test -p tablerock-engine --lib`
- `cargo test -p tablerock-engine --test engine_service --test driver_runtime --test session_registry`
- CI container-free job now selects those engine integration targets.

## Remaining work

- Arbitrary PG/CH statement streaming + health (plan 002 step 3).
- Multi-operation real-server proof (step 4).
