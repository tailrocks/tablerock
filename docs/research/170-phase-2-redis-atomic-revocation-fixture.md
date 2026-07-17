# Phase 2 Redis Atomic Revocation Fixture

## Failure found by the workspace gate

The serialized real-server suite exposed a race in its TLS credential-revocation
fixture. It killed the channel user's connections, awaited that reply, then sent
a separate command to kill the pattern user's connections. The first kill could
start stale-credential reconnect activity before the second administrative
command reached Redis, causing the second command to time out.

This was fixture sequencing, not product retry behavior. Retrying the second
administrative command would preserve the race and weaken the proof.

## Structural repair

The fixture now sends both `CLIENT KILL USER` commands in one redis-rs pipeline
and validates both integer replies. Both commands reach the server before either
reply is awaited, removing the inter-command reconnect window while preserving
independent killed-connection counts.

The complete Redis 7.4.9/8.8.0, RESP2/RESP3, TLS/mTLS authentication and active
channel/pattern revocation matrix passes with the repaired fixture. Product
behavior remains unchanged: each affected subscription stops with bounded
redacted authentication truth and no false recovery gap.

Context7 was attempted and reported its monthly quota exhausted. Pipeline
behavior was verified against pinned redis-rs 1.4.0 source and existing
TableRock real-server pipeline tests.

External concepts: Redis command pipelining and CLIENT KILL USER sequencing
Public sources: <https://docs.rs/redis/1.4.0/redis/struct.Pipeline.html>, <https://redis.io/docs/latest/commands/client-kill/>
Implementation source: TableRock-owned Testcontainers fixture
Copied code/assets/text: none
