# Phase 2 Redis Timeout and Reconnect Evidence

Date: 2026-07-17

## Decision

Normal Redis commands and cancellation control use redis-rs 1.4.0
`ConnectionManager` behind the private adapter. `RedisRuntimePolicy` owns five
finite facts: connection timeout, response timeout, reconnect-attempt count,
minimum backoff, and maximum backoff. Zero durations, zero attempts, more than
32 attempts, any duration above five minutes, and inverted backoff bounds are
rejected before I/O. The five-minute ceiling prevents practically unbounded
configuration and makes deadline construction overflow-safe. The default
policy is a five-second connection timeout, 30-second response timeout, eight
attempts, and 100-millisecond through two-second jittered exponential backoff.

The selected manager never replays the command that detects a dropped
connection. It returns that failure and reconnects only future calls. RESP3 can
observe a disconnect before the next request and proactively reconnect, so the
next call may succeed without first surfacing an error. Either path preserves
the configured logical database because reconnect derives from the original
client configuration.

Rust exposes message-free `Timeout` and `Connection` categories separately from
server `Command` rejection. A timeout does not prove whether a future write was
applied and carries no automatic retry permission. TableRock currently exposes
only read operations through this Redis adapter; later write execution must map
post-dispatch timeout/drop to unknown outcome and never replay it.

Blocking commands do not use the normal managed connection. Each blocking
operation opens a fresh disposable connection with a bounded connect timeout,
no BLPOP response timeout, concurrency one, and buffer one. Its nonblocking
CLIENT ID setup handshake uses the finite response timeout. One generation-
scoped operation token owns identity, cancellation request, confirmation, and
release, so a stale stream cannot clear or confirm a newer operation.
Cancellation during setup marks the operation and returns the distinct
`PreventedBeforeDispatch` fact; the resumed setup then prevents BLPOP dispatch
and truthfully terminates as `ClientStopped`. After identity publication, the dispatch path retries CLIENT
UNBLOCK within the response deadline to close the identity-to-block-registration
race. Cancellation uses a separate bounded
managed control connection. Completion, failure, or drop releases only its own
token. This removes stale-ID and split-atomic ABA conditions.

## Evidence

Testcontainers Rust 0.27.3 runs immutable official Redis 7.4.9 and 8.8.0
images. Both RESP2 and RESP3 prove:

- `CLIENT PAUSE 300 ALL` makes a read exceed the configured 100-millisecond
  response bound and map to `Timeout`;
- a later explicit read succeeds after the pause, without an internal replay
  loop in TableRock;
- `CLIENT KILL ID` drops the observed normal connection;
- RESP2 returns `Connection` from the detecting read, while RESP3 may already
  have proactively reconnected;
- future explicit reads reconnect within five seconds and preserve the binary
  value in logical database 1; and
- immediate cancellation after service start remains reachable during blocking
  setup and terminates as `ClientStopped`, while post-dispatch cancellation is
  separately proven server-confirmed; and
- blocking completion/cancellation plus stale-release unit evidence pass with
  disposable generation-scoped ownership and active-ID cleanup.

An inert TCP peer proves the RESP3 connection/handshake path returns `Timeout`
inside the configured bound. Unit tests prove every runtime-policy boundary,
exact manager projection, stable adapter error mapping, and stale-token safety.
This closes the Phase 2 Redis response-timeout and dropped-connection reconnect
tracer. TLS/authentication is subsequently closed by
[research 144](144-phase-2-redis-tls-authentication.md). DNS changes, server
restart during active work, Pub/Sub resubscription, strict pre-decode transport allocation, reviewed TTL
mutation, complete type views, service/UI integration, and native presentation
remain open.

Context7 was attempted first and reported its monthly quota exhausted. The
redis-rs behavior and configuration were verified from exact pinned 1.4.0
source. Its primary API documentation states that the detecting failure is
returned, reconnection is installed for future commands, and RESP3 actively
observes disconnects.

## Provenance

External concept: bounded Redis command timing and future-call reconnection  
Public sources: <https://docs.rs/redis/1.4.0/redis/aio/struct.ConnectionManager.html>,
<https://docs.rs/redis/1.4.0/redis/aio/struct.ConnectionManagerConfig.html>, and
<https://redis.io/docs/latest/commands/client-kill/>  
TableRock requirements: research 03, 06, 10, 14, 20, 30, 31, 32, 51, and 90  
Implementation source: TableRock-owned policy, adapter ownership, and
independent Testcontainers fixtures  
Copied code/assets/text: none
