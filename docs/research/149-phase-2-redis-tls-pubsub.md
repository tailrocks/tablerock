# Phase 2 Redis TLS Pub/Sub Composition Evidence

Date: 2026-07-17

## Decision

Redis Pub/Sub uses the same Rust-owned client construction as ordinary commands.
Dedicated RESP2 and RESP3 subscription connections therefore inherit required
TLS, custom trust roots, optional client identity, hostname verification, ACL
credentials, protocol selection, logical database configuration, timeouts, and
redaction without a second security path.

ACL command permission and channel permission are independent Redis facts.
TableRock never widens server authorization. The synthetic all-access fixture
explicitly grants `&*` in addition to `~* +@all`; this fixes test intent rather
than bypassing production enforcement. Restricted-channel denial remains open
until its Redis-version semantics and adapter outcome have dedicated evidence.

Channel and pattern selectors remain binary and bounded before TLS I/O. Payloads
retain the same pre-queue and page bounds. Cancellation sends the matching
`UNSUBSCRIBE` or `PUNSUBSCRIBE` over the authenticated dedicated connection and
still terminates as client-stop truth.

## Evidence

Testcontainers Rust runs immutable official Redis 7.4.9 and 8.8.0 TLS-only
servers. Under RESP2 and RESP3, with both server-authenticated TLS and required
mutual TLS, the matrix proves custom-root and hostname-verified setup,
ACL-authenticated binary channel and pattern delivery, exact two- and
three-column pages, configured all-channel permission, and server-observed
authenticated teardown.

The existing negative matrix continues to prove missing required client
identity, wrong password, wrong root, hostname mismatch, and plaintext fallback
fail closed.

This closes TLS/mTLS/ACL composition for channel and pattern Pub/Sub.
Restricted-channel denial, DNS changes,
strict RESP2 pre-decode allocation bounds,
presentation, and clean-machine release evidence remain open. Research 150
records the restricted-denial server evidence and official-client blocker.
Research 151 closes revocation for future ordinary operations.
Research 152 closes active channel- and pattern-subscription revocation.
Research 153 closes TLS/mTLS server-replacement resubscription.

Context7 library documentation was already selected as `/redis-rs/redis-rs`;
the pinned redis-rs 1.4.0 client construction and official Redis ACL channel
rules were verified by the real-server matrix.

## Provenance

External concept: Redis TLS, ACL channel patterns, and Pub/Sub
Public sources: <https://redis.io/docs/latest/operate/oss_and_stack/management/security/acl/>
and <https://redis.io/docs/latest/develop/pubsub/>
TableRock requirements: research 06, 10, 14, 20, 30, 31, 32, 53, 144, 145,
147, and 148
Implementation source: TableRock-owned security construction and bounded stream
contracts
Copied code/assets/text: none
