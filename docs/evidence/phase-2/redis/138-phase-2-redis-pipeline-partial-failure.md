# Phase 2 Redis Pipeline Partial-Failure Evidence

Date: 2026-07-17

## Decision

The disposable real-server harness now proves individual command outcomes when
one server-side response fails inside a pipeline. It uses the selected `redis`
1.4.0 client's `Pipeline::ignore_errors()` contract. It never parses wire
replies or introduces another Redis client.

One test-only three-command sequence executes `SET`, an intentionally
type-invalid `HSET`, and `INCR` against one container-scoped fixture key. Local
test outcomes distinguish server success from rejection, and an independent
`GET` observes the final counter. A successful cleanup deletes the fixture key
before the surrounding SCAN assertion. No pipeline command, response, error, or
mutation surface was added to the production engine API.

Both execution modes are explicit:

- `Pipelined` sends the three commands as one ordinary non-transactional
  pipeline.
- `MultiExec` encloses them in `MULTI`/`EXEC`. This means isolated sequential
  execution, not rollback. A runtime type error does not undo the first command
  or prevent the third command.

The response must contain exactly three entries. A queue, transport, protocol,
or cleanup failure fails the disposable test and never becomes product outcome
semantics. This feasibility evidence has no cancellation claim and exposes no
arbitrary-command bypass. Later production pipeline execution must consume
reviewed Rust-owned mutation authority and preserve ambiguous-write and
post-dispatch cancellation truth before any public API exists.

## Real-server matrix

Testcontainers Rust 0.27.3 runs the existing immutable official Redis 7.4.9
and 8.8.0 images. Each line passes RESP2 and RESP3 in both pipeline modes. Every
case observes `[ServerSucceeded, ServerRejected, ServerSucceeded]` and final
counter `2`, proving that the response error neither stops later execution nor
rolls back successful commands. The following SCAN assertion also proves
cleanup left no fixture key.

This closes the Phase 2 pipeline/partial-response feasibility item. TLS,
authentication, HSCAN/SSCAN/ZSCAN, TTL, Pub/Sub, timeout, reconnect, and broader
post-dispatch failure races remain open.

Context7 was attempted first and reported its monthly quota exhausted. The
exact pinned redis-rs source documents `ignore_errors()` and its async pipeline
behavior. Redis primary documentation defines ordered pipeline replies,
transaction runtime errors, continued execution, and the absence of rollback.

## Provenance

External concepts: Redis pipelining, per-command response errors, and
`MULTI`/`EXEC` no-rollback semantics  
Public sources: <https://docs.rs/redis/1.4.0/redis/struct.Pipeline.html>,
<https://redis.io/docs/latest/develop/using-commands/pipelining/>, and
<https://redis.io/docs/latest/develop/using-commands/transactions/>  
TableRock requirements: research 03, 10, 20, 30, 31, 32, 90, and 104  
Implementation source: TableRock-owned test-only fixture helper and independent
Testcontainers servers  
Copied code/assets/text: none
