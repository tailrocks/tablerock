# Phase 2 Redis Reviewed TTL Mutation Evidence

Date: 2026-07-17

## Decision

Redis TTL mutation consumes one move-only `AuthorizedMutationPlan` produced by
the bounded, exact-once review registry. The adapter accepts exactly one
`RedisSetExpiration` change targeting the connected logical database. `Persist`
maps to `PERSIST`; `ExpireAfterMillis` maps to `PEXPIRE`. Preserve, mixed or
multi-command plans, non-Redis targets, and logical-database mismatches fail
before I/O.

One reviewed action maps to one Redis command. This avoids hiding partial
success behind a scalar result; the future general Redis mutation executor must
report every sequential command independently. Expirations are positive and no
larger than Redis's signed 64-bit integer range. `RedisConnectConfig` therefore
uses the same nonnegative `u32` logical-database identity as `MutationTarget`
instead of retaining a competing signed representation.

The closed result is `Applied` or `NotApplied`. `PEXPIRE` zero means the target
was absent. `PERSIST` zero deliberately does not guess between an absent key and
an already-persistent key. Presentation must refresh the typed TTL fact when it
needs current state.

Transport loss or timeout after dispatch maps to `WriteOutcomeUnknown`; it is
never converted to failure and never retried automatically. Authentication and
server command rejection remain distinct stable failures. Mutation IDs and
review-token IDs cross the result boundary, but keys, values, command text, and
driver diagnostics do not.

## Evidence

Testcontainers Rust runs immutable official Redis 7.4.9 and 8.8.0 images under
RESP2 and RESP3. The matrix proves:

- missing-key expiry and already-persistent `PERSIST` are `NotApplied`;
- positive millisecond expiry and subsequent persistence are `Applied` and
  agree with typed `PTTL` observations;
- exact-once registry authorization precedes execution;
- database mismatch, unsupported changes, and multi-command TTL plans fail
  without mutation;
- binary-safe key arguments are never interpolated; and
- a server-paused write times out as `WriteOutcomeUnknown`, then is observed as
  applied, proving why automatic retry would be unsafe.

This closes the reviewed key-level TTL mutation tracer. Conditional NX/XX/GT/LT
UX, hash-field TTLs, the general sequential Redis mutation executor, service and
UniFFI mutation ownership, live credential-revocation races, and both
presentations remain open.

Context7 selected `/redis-rs/redis-rs`; behavior was cross-checked against
redis-rs 1.4.0 and the official Redis command references.

## Provenance

External concept: Redis millisecond expiration and persistence commands
Public sources: <https://redis.io/docs/latest/commands/pexpire/> and
<https://redis.io/docs/latest/commands/persist/>
TableRock requirements: research 06, 10, 14, 30, 31, 32, 51, 104, 106, and 139
Implementation source: TableRock-owned reviewed mutation and closed outcome
contracts
Copied code/assets/text: none
