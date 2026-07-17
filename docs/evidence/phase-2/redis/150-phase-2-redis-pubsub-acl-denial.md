# Phase 2 Redis Pub/Sub ACL Denial Boundary

Date: 2026-07-17

## Decision

TableRock must not report a Redis subscription as established until Redis has
accepted its channel or pattern. A restricted-channel `NOPERM` response is a
command failure; it is never an empty subscription, retryable disconnect, or
successful long-lived operation.

The latest released official client remains `redis-rs` 1.4.0. TableRock will
not vendor that crate, hand-write RESP, or use an administrative `ACL DRYRUN`
preflight as a production authorization substitute. Those approaches would
either retain a private client fork, introduce a competing protocol stack, or
require privileges ordinary profiles do not possess.

The restricted-denial adapter gate remains open. Its required resolution is an
official-client path that returns the actual subscription acknowledgement, or
an upstream redis-rs correction that converts `Value::ServerError` before its
Pub/Sub setup methods discard the reply value.

## Measured boundary

Official Redis documentation defines `&pattern` as the allowed channel set.
`SUBSCRIBE` channel names use glob matching; `PSUBSCRIBE` requires its submitted
pattern to match an allowed ACL pattern literally.

Testcontainers Rust now starts immutable Redis 7.4.9 and 8.8.0 TLS-only servers
with `acl-pubsub-default resetchannels` and a synthetic restricted user limited
to `&allowed:*`. Across RESP2 and RESP3, ordinary TLS and required mutual TLS,
`ACL DRYRUN` returns a normal string explaining that `denied:channel` is not
permitted. The test decodes and asserts that string instead of treating command
transport success as authorization success.

Direct inspection of redis-rs 1.4.0 and current upstream `main` at
`67685eb28be79e05f26e75b5f36a403610a56fad` found the same Pub/Sub setup path:
`send_recv(...).await.map(|_| ())`. Redis protocol errors are represented as
`Value::ServerError`; mapping every reply to unit erases `NOPERM`. A real RESP2
restricted subscription consequently returned an apparently successful stream
that never produced a page. That behavior is evidence of an open dependency
boundary, not an accepted TableRock contract.

## Failure and safety contract

- Never infer authorization from an idle stream.
- Never publish a probe message to test subscription permission.
- Never require administrative ACL introspection for normal profiles.
- Never retry `NOPERM`; return a bounded command failure before stream handoff.
- Keep the session registry and long-operation ownership unclaimed on denial.
- Preserve redaction: ACL usernames, patterns, credentials, and server detail do
  not enter default logs or presentation errors.

## Remaining work

Restricted channel and pattern denial through the TableRock adapter remains
open for RESP2 and RESP3. Allowed restricted-channel delivery, reconnect after
authorization changes, DNS
changes, strict RESP2 pre-decode allocation bounds, and presentation also
remain open. Research 151 subsequently closes revocation for future ordinary
operations.
Research 152 closes active channel- and pattern-subscription revocation.

## Provenance

External concepts: Redis ACL channel authorization and redis-rs Pub/Sub reply
handling
Public sources:
<https://redis.io/docs/latest/operate/oss_and_stack/management/security/acl/>,
<https://redis.io/docs/latest/commands/acl-dryrun/>, and
<https://github.com/redis-rs/redis-rs/blob/67685eb28be79e05f26e75b5f36a403610a56fad/redis/src/aio/pubsub.rs>
TableRock requirements: research 01, 06, 10, 14, 20, 30, 31, 32, 145, 147,
148, and 149
Implementation source: TableRock-owned Testcontainers fixture and bounded
adapter requirements
Copied code/assets/text: none
