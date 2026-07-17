# Phase 2 Redis Subscription Connect Policy

## Failure class

The exhaustive TLS replacement gate exposed two outcomes from one connection
phase. An invalid replacement identity could fail immediately as `Connect` or
consume the connection deadline as `Timeout`. It also exposed that initial
dedicated Pub/Sub setup made only one connection attempt after the ordinary
session became ready, while reconnect used the configured bounded attempt set.

The architecture allowed scheduling and TLS-handshake timing to change a stable
public outcome and gave initial and replacement generations different ownership
rules.

## Decision

Initial and replacement RESP2/RESP3 subscription generations now share one
bounded, cancellable connection-attempt policy. Authentication and protocol
failures remain terminal. Backoff and attempt bounds remain the immutable
session policy. Initial setup never emits a delivery discontinuity; only a
successfully restored generation may do that.

For required-TLS subscription connections, connection-phase deadline exhaustion
maps to the redacted `Connect` class, matching immediate TLS transport or trust
failure. Plaintext blackhole connection attempts remain `Timeout`. Initial
ordinary session handshake behavior is unchanged.

## Evidence

- plaintext RESP2/RESP3 blackhole attempts exhaust within their bound as
  `Timeout`;
- invalid-trust and rotated-credential replacement passes Redis 7.4.9/8.8.0,
  RESP2/RESP3, channel/pattern, server TLS/required-mTLS combinations;
- untrusted replacements terminate as `Connect`, rotated credentials as
  `Authentication`, and rejected generations emit no false recovery gap;
- initial dedicated connection scheduling uses the same bounded retry and
  cancellation ownership without announcing recovery.

Context7 was attempted and reported its monthly quota exhausted. redis-rs API
behavior was verified against pinned 1.4.0 source and direct Testcontainers
tests.

External concepts: Redis Pub/Sub dedicated connections, TLS handshake classification, bounded reconnect
Public sources: <https://docs.rs/redis/1.4.0>, <https://redis.io/docs/latest/develop/pubsub/>
Implementation source: TableRock-owned Redis subscription state machine and Testcontainers fixtures
Copied code/assets/text: none
