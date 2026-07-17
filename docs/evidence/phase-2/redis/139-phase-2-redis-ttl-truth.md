# Phase 2 Redis TTL Truth

Date: 2026-07-17

## Decision

Rust core now owns `RedisTimeToLive`, a closed point-in-time fact with three
non-overlapping states:

- `Missing`: the key did not exist when Redis evaluated the request;
- `Persistent`: the key existed without expiration; and
- `Expiring { remaining_millis }`: the key existed with a finite remaining
  lifetime measured in milliseconds.

This preserves Redis's `PTTL` distinctions instead of collapsing both negative
sentinels into `None`. The contract exposes `key_existed_at_observation()` and
`remaining_millis()` without any redis-rs type or wire sentinel. It makes no
promise that the observed key still exists after the reply.

The Redis adapter issues the read-only, O(1) `PTTL` command with raw key
bytes. It maps `-2`, `-1`, and nonnegative replies to the core states and rejects
every other negative integer as `Protocol`. Transport/server failure remains the
message-free `Command` category. The operation performs no write, has no
ambiguous-write state, and adds no independent cancellation claim.

## Evidence

Core tests prove missing, persistent, and expiring states remain distinguishable
and that only the expiring state has milliseconds. Adapter tests reject an
undocumented negative reply.

Testcontainers Rust 0.27.3 runs immutable official Redis 7.4.9 and 8.8.0 images.
Under both RESP2 and RESP3, each line proves:

- a missing non-UTF-8 binary key maps to `Missing`;
- an existing key without expiration maps to `Persistent`; and
- an existing key with a ten-minute fixture expiration maps to a finite value
  from 1 through 600,000 milliseconds.

This closes Phase 2 key-level TTL read truth. Reviewed TTL mutation semantics,
field-level expiration commands, expiration races, TLS, authentication,
HSCAN/SSCAN/ZSCAN, Pub/Sub, timeout, and reconnect remain open.

Context7 was attempted first and reported its monthly quota exhausted. The
redis-rs query API was verified from exact pinned source; Redis primary
documentation defines `PTTL` as read-only and specifies all three replies for
RESP2 and RESP3.

## Provenance

External concept: Redis millisecond TTL observation  
Public sources: <https://redis.io/docs/latest/commands/pttl/> and
<https://docs.rs/redis/1.4.0>  
TableRock requirements: research 03, 10, 14, 20, 30, 31, 32, and 90  
Implementation source: TableRock-owned core fact, adapter mapping, and
independent Testcontainers fixtures  
Copied code/assets/text: none
