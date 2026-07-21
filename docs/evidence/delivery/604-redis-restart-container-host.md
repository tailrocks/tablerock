# Redis restart fixture container host

Date: 2026-07-22

## Failure class

Velnor run 29856858353 passed format, lint, dependency audit, and the initial
36 real-server cases. `resubscribes_with_visible_gap_after_redis_restart` then
timed out during its first adapter connection. Unlike the ordinary Redis
fixtures corrected by evidence 600, this plaintext restart fixture creates
containers inline and had not registered Testcontainers' authoritative host.
It therefore combined a remote Docker mapped port with `127.0.0.1`.

## Correction

The restart fixture records the container host after both initial and
replacement startup. Adapter and publisher connections continue resolving the
host by mapped port, including across the same-port replacement boundary.
TLS replacement fixtures also drop and recreate their local forward at the
replacement boundary, preventing accepted connections owned by the old
container from surviving into readiness checks. Production Redis connection
behavior is unchanged.

## Verification

- `cargo fmt --all -- --check`
- `cargo nextest run -p tablerock-engine --test redis_real resubscribes_with_visible_gap_after_redis_restart -- --exact --nocapture`
  passed locally across all eight version/protocol/subscription combinations.
- `cargo nextest run -p tablerock-engine --test redis_real resubscribes_with_visible_gap_after_tls_redis_restart -- --exact --nocapture`
  passed locally across all 16 TLS restart combinations in 28.81 seconds.
- `cargo nextest run -p tablerock-engine --test redis_real rejects_untrusted_or_recredentialed_tls_pubsub_replacement -- --exact --nocapture`
  passed all 32 hostile replacement combinations in 353.61 seconds.
- Exact-main Velnor proof remains required after push.
- Velnor run 29860746521 exposed the replacement forward's intentionally
  lifetime-only ownership binding under `-D warnings`; it is named as an
  ownership guard so workspace Clippy can verify the test target.

## Provenance

No external product reference influenced this test correction. It follows the
existing Testcontainers endpoint contract documented in evidence 594 and 600.
