# Phase 2 Redis TLS Pub/Sub Reconnect Evidence

Date: 2026-07-17

## Decision

Redis Pub/Sub reconnect retains the original immutable security configuration.
Every replacement connection repeats hostname verification, custom-root
validation, optional client-identity presentation, ACL authentication, protocol
selection, and subscription acknowledgement before becoming active.

A successful reconnect never implies lossless delivery. TableRock emits one
ordered zero-row `DeliveryDiscontinuity` page before any restored message.
Authentication or TLS failure is terminal; transport failure alone follows the
bounded reconnect policy.

## Evidence

Testcontainers Rust runs immutable Redis 7.4.9 and 8.8.0. The matrix covers
RESP2 and RESP3, channel and pattern subscriptions, and both server-authenticated
TLS and required mutual TLS.

Each case binds the first TLS-only server to a fixed host endpoint, establishes
an ACL-authenticated bounded binary subscription, removes that server, and
starts a fresh TLS-only server with the same certificate and ACL fixture on the
same endpoint. Publishing waits until the replacement reports one subscriber.
The stream then requires an exact zero-row discontinuity page, a following exact
binary message page, and prompt client cancellation within one second.

The initial and replacement servers share one TLS fixture factory, preventing
drift in certificates, client identity, ACLs, or restrictive Pub/Sub defaults.

This closes TLS/mTLS server-replacement resubscription for channels and
patterns. DNS endpoint changes, replacement with invalid trust or credentials,
restricted subscription denial, strict RESP2 pre-decode allocation bounds, and
presentation remain open.

## Safety contract

- No message can overtake the reconnect discontinuity page.
- No reconnect automatically replays ordinary commands or mutations.
- TLS and ACL failures never degrade to plaintext or anonymous access.
- Cancellation interrupts reconnect and releases generation ownership.
- Credentials, selectors, payloads, and TLS material remain redacted.

## Provenance

External concepts: Redis TLS, ACL authentication, and Pub/Sub delivery semantics
Public sources: <https://redis.io/docs/latest/operate/oss_and_stack/management/security/encryption/>,
<https://redis.io/docs/latest/operate/oss_and_stack/management/security/acl/>,
and <https://redis.io/docs/latest/develop/pubsub/>
TableRock requirements: research 01, 06, 10, 14, 20, 30, 31, 32, 144, 145,
148, 149, 151, and 152
Implementation source: TableRock-owned TLS fixture and bounded subscription
state machine
Copied code/assets/text: none
