# Phase 2 Redis Pub/Sub Credential Revocation Evidence

Date: 2026-07-17

## Decision

An active Redis subscription whose credential is revoked must terminate with a
redacted authentication failure. It must not remain idle, emit a delivery-gap
page, exhaust generic connection retries, or accept messages under obsolete
authorization.

The existing bounded resubscription loop remains valid for transport loss. An
authentication failure during reconnect is terminal and bypasses further
backoff attempts. No subscription generation becomes active until connection
and subscription setup both succeed.

## Evidence

The immutable Redis 7.4.9/8.8.0 Testcontainers matrix runs RESP2 and RESP3 with
server-authenticated TLS and required mutual TLS. Each fixture establishes
bounded channel and pattern subscriptions under independent users, requires
`PUBSUB NUMSUB` and `PUBSUB NUMPAT` to observe exactly one registration each,
rotates both passwords, and requires `CLIENT KILL USER` to terminate each
user's established sockets.

Both pending pages then terminate as `RedisError::Authentication` within five
seconds. The ordinary command connection independently reaches the same result,
proving channel, pattern, and shared manager paths reject obsolete credentials.

This closes active channel- and pattern-subscription credential revocation.
Restricted subscription denial, DNS changes, TLS server
replacement during subscription, strict RESP2 pre-decode allocation bounds,
and presentation remain open.

## Safety contract

- Authentication failure never becomes a delivery discontinuity.
- Obsolete credentials are never retried after their terminal classification.
- No message, selector, username, password, or Redis response detail enters the
  stable error or default logs.
- Recovery requires a newly constructed session with newly resolved secret
  material.

## Provenance

External concepts: Redis ACL rotation, client termination, and Pub/Sub reconnect
Public sources: <https://redis.io/docs/latest/commands/acl-setuser/>,
<https://redis.io/docs/latest/commands/client-kill/>, and
<https://redis.io/docs/latest/develop/pubsub/>
TableRock requirements: research 01, 06, 10, 14, 20, 30, 31, 32, 145, 148, 149,
150, and 151
Implementation source: TableRock-owned subscription state machine and TLS
Testcontainers fixture
Copied code/assets/text: none
