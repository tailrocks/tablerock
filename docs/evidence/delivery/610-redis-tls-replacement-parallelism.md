# Redis TLS replacement case isolation

## Requirement

Redis 7.4 and 8.8 must reject both recredentialed and untrusted TLS Pub/Sub
replacement connections across RESP2/RESP3, server-only/mutual TLS, and
channel/pattern subscriptions. CI must expose each case independently and let
nextest schedule independent cases concurrently.

## Architectural cause

The coverage lived in one Tokio test containing five nested loops. That test
serialized all 32 independent Docker-backed cases, hid the active combination
from nextest reporting, and made one slow case look like a stalled suite.
Nextest can parallelize only test functions, not iterations inside one test.

## Delivery

- Replaced the nested-loop test with 32 explicitly named Tokio tests generated
  by one local macro.
- Kept the shared assertion helper and full Cartesian coverage unchanged.
- Names encode Redis version, protocol, identity mode, subscription kind, and
  rejection reason so failures identify the exact contract case.

## Verification

```text
cargo fmt --all -- --check
cargo clippy -p tablerock-engine --test redis_real --locked -- -D warnings
cargo nextest list -p tablerock-engine --test redis_real --locked -T oneline
cargo nextest run -p tablerock-engine --test redis_real --locked -j 4 \
  redis74_resp2_server_channel_recredentials \
  redis74_resp3_mutual_pattern_untrusted \
  redis88_resp2_mutual_channel_untrusted \
  redis88_resp3_server_pattern_recredentials
```

Discovery reports all 32 replacement cases. Representative cross-matrix run:
4 passed, 40 skipped, 46.792 seconds.
