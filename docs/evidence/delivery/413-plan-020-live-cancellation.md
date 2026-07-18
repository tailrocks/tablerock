# Plan 020 — live Swift cancellation and bridge lock fix

Date: 2026-07-19

## Checkpoint

The cancellation audit found a structural defect: synchronous `pump` held the
coarse bridge mutex while awaiting the next driver event. `cancel` needs that
same mutex to mutate service cancellation state, so a slow query could prevent
its own cancellation until completion.

`pump` now bounds each event wait to 10 ms and releases/yields the bridge mutex
between polls. This retains the synchronous coarse UniFFI API while allowing a
concurrent operation-ID cancellation to acquire service state promptly. No
secondary runtime, RPC path, or Swift-side cancellation semantics were added.

The strict Swift behavior harness now submits PostgreSQL `pg_sleep(10)`, pumps
on a detached task, requests cancellation after 150 ms, and requires runtime
dispatch, `cancel_dispatched`, an honest cancellation terminal, and total
latency below three seconds.

## Evidence

| Gate | Observation |
|------|-------------|
| `cargo test -p tablerock-ffi` | 17 passed, 5 ignored |
| live PostgreSQL slow query | `pg_sleep(10)` cancellation PASS in 0.177 s |
| core outcome | `Requested` |
| runtime outcome | `Queued` |
| event truth | `cancel_dispatched` then `server_confirmed_cancelled` |
| full native behavior matrix | query + catalog PASS on PostgreSQL 18.4, ClickHouse 25.8, Redis 8.0 after lock fix |
| Swift gate | Swift 6 complete concurrency and warnings-as-errors |

## Bounds

This proves the exact bridge/actor cancellation path used by the app and fixes
the condition that prevented concurrent dispatch. macOS assistive-access denial
still prevents automated clicking of the toolbar Cancel button, so UI click
automation is not claimed. ClickHouse and Redis retain their already documented
engine-specific cancellation truth; this checkpoint's slow-query terminal proof
is PostgreSQL-specific.
