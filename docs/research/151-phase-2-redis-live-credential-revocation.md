# Phase 2 Redis Live Credential Revocation Evidence

Date: 2026-07-17

## Decision

Redis password rotation does not retroactively invalidate an authenticated
socket. TableRock defines live revocation as two server facts: replace the
credential, then terminate the old user's connections. The next operation may
first observe connection loss, but reconnect with the obsolete credential must
stop as a redacted authentication failure within the configured bound.

TableRock does not replay commands, retry an obsolete password through a
subscription backoff loop, fall back to plaintext, or expose Redis response
detail. Recovery requires an explicit new session built from newly resolved
secret material; an existing session never mutates its credential in place.

## Evidence

Testcontainers Rust runs immutable official Redis 7.4.9 and 8.8.0 TLS-only
servers under RESP2 and RESP3, with both server-authenticated TLS and required
mutual TLS. Each fixture establishes a TableRock session, rotates its ACL
password through the authenticated administrator connection, terminates every
connection owned by that user with `CLIENT KILL USER`, then requires the next
read-only identity operation to reach `RedisError::Authentication` within five
seconds.

The fixture uses two-second connection/response bounds and two reconnect
attempts. At least one terminated connection is required, so the test cannot
pass without exercising an established authenticated session.

This closes live credential revocation for future ordinary Redis operations.
Research 152 subsequently closes active channel- and pattern-subscription
revocation. DNS changes, restricted subscription denial,
strict RESP2 pre-decode allocation bounds, secret re-resolution presentation,
and clean-machine evidence remain open.

## Safety contract

- Password replacement alone is not claimed as socket revocation.
- Confirmed server-side connection termination begins reconnect evidence.
- No mutation is automatically replayed after connection loss.
- Authentication and credential detail stay absent from stable errors, `Debug`,
  logs, pages, and persistence.

## Provenance

External concepts: Redis ACL password rotation and user-connection termination
Public sources: <https://redis.io/docs/latest/commands/acl-setuser/> and
<https://redis.io/docs/latest/commands/client-kill/>
TableRock requirements: research 01, 06, 10, 14, 20, 30, 31, 32, 53, 143, 144,
149, and 150
Implementation source: TableRock-owned TLS fixture, runtime policy, and redacted
error contract
Copied code/assets/text: none
