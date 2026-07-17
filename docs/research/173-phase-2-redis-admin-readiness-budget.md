# Phase 2 Redis Administrative Readiness Budget

## Repeated gate failure

Sustained workspace runs exposed timeouts in otherwise unrelated TLS Redis
fixture commands: `PUBLISH`, `PUBSUB NUMSUB`, and a pipelined pair of
`CLIENT KILL USER` operations. The common path was the raw redis-rs
administrative connection helper.

Pinned redis-rs 1.4.0 defaults raw async connections to a 500 ms response
timeout and one-second connection timeout. The fixture returned immediately
after TLS connection establishment, without proving Redis command readiness.
Docker scheduling could therefore turn ordinary fixture administration into a
false product failure.

## Structural repair

Raw TLS administrative connections now use explicit five-second connection and
response budgets and return only after `PING` receives `PONG`. The enclosing
fixture readiness loop remains capped at fifteen seconds. Product session
policies and their tighter timeout evidence are unchanged.

The complete eight-test Redis real-server suite passes across Redis 7.4.9 and
8.8.0, RESP2/RESP3, TLS/mTLS, restart, replacement, revocation, scanning,
mutation, cancellation, and timeout paths. Administrative mutation commands are
still never replayed; readiness is established before they are issued.

Context7 was attempted and reported its monthly quota exhausted. Default and
configured timeout behavior was verified against pinned redis-rs 1.4.0 source.

External concepts: redis-rs async connection budgets and Redis PING readiness
Public sources: <https://docs.rs/redis/1.4.0/redis/struct.AsyncConnectionConfig.html>, <https://redis.io/docs/latest/commands/ping/>
Implementation source: TableRock-owned Testcontainers fixture
Copied code/assets/text: none
