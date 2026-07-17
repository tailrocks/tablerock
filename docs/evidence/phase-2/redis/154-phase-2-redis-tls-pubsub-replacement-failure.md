# Phase 2 Redis TLS Pub/Sub Replacement Failure Evidence

Date: 2026-07-17

## Decision

A Redis replacement subscription generation is committed only after transport,
TLS, client identity, ACL authentication, and subscription setup all succeed.
Failure before that point is terminal after the bounded reconnect policy and
must not emit `DeliveryDiscontinuity`; that warning means delivery resumed and
there may have been loss, not merely that an unsuccessful attempt occurred.

An untrusted replacement maps to the redacted connect class. A replacement that
no longer accepts the immutable session credential maps to the redacted
authentication class. Neither condition retries commands, falls back to
plaintext, or exposes server detail.
Research 171 makes this classification timing-independent: required-TLS
connection-phase deadline exhaustion maps to the same `Connect` class as an
immediate TLS validation failure, while plaintext blackholes remain `Timeout`.

## Evidence

Testcontainers Rust runs Redis 7.4.9 and 8.8.0 across RESP2/RESP3, channel and
pattern subscriptions, and server-authenticated TLS/required mTLS. For every
combination, the initial server must expose exactly one subscription through
`PUBSUB NUMSUB` or `PUBSUB NUMPAT` before removal.

Two independent replacement cases then run on the same fixed endpoint:

- a fresh server identity signed by an untrusted CA must terminate the pending
  page as `RedisError::Connect`;
- the trusted server identity with a rotated ACL password must terminate it as
  `RedisError::Authentication`.

Both outcomes are required within 60 seconds under the bounded 32-attempt
replacement policy; its one-second connection timeout plus 500 ms maximum
backoff has a 48-second conservative ceiling. A 250 ms minimum backoff prevents fast
connection-refused results from exhausting the policy during the intentional
same-endpoint container replacement window. Because `next_page` returns the
terminal error directly,
the test also proves no zero-row discontinuity page was queued by a rejected
generation.

Fixed-host-port Testcontainers readiness is verified at the Redis protocol
boundary after the container log wait. Adapter setup retries only redacted
connect, connection-loss, and timeout availability failures for at most fifteen
seconds; authentication, TLS-configuration, and protocol failures remain
immediate. Raw TLS fixture setup uses the same bounded protocol-readiness rule.
Negative replacement fixtures additionally prove the new server is
protocol-ready using its valid admin trust and credentials before asserting the
subscriber's old trust/credential outcome. This prevents Docker port-publication
timing from masquerading as a product reconnect failure.

The exhaustive TLS fixture uses one-second connection and response bounds so
container scheduling cannot masquerade as initial subscription failure. These
are verification-harness budgets, not product defaults.
Initial dedicated Pub/Sub setup now shares the bounded connection-attempt policy
with replacement generations, but never emits a recovery-gap marker.

This closes invalid-trust and invalid-credential replacement behavior for TLS
channel and pattern subscriptions. DNS endpoint changes, restricted initial
subscription denial, strict RESP2 pre-decode allocation bounds, presentation,
and clean-machine evidence remain open.

## Safety contract

- Only a fully authenticated and subscribed generation can announce recovery.
- TLS trust failure and ACL failure remain distinct stable redacted classes.
- Failed generations cannot send messages or mutate active ownership.
- Credentials, certificate material, selectors, and Redis response detail are
  absent from errors and default logs.

## Provenance

External concepts: Redis TLS identity, ACL authentication, and Pub/Sub reconnect
Public sources: <https://redis.io/docs/latest/operate/oss_and_stack/management/security/encryption/>,
<https://redis.io/docs/latest/operate/oss_and_stack/management/security/acl/>,
and <https://redis.io/docs/latest/develop/pubsub/>
TableRock requirements: research 01, 06, 10, 14, 20, 30, 31, 32, 144, 148, 149,
153
Implementation source: TableRock-owned TLS fixture and subscription generation
guard
Copied code/assets/text: none
