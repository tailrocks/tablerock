# Evidence 651: Redis restart fixture port-race removal

## Claim

The real Redis Pub/Sub restart test no longer assumes that a host port remains
available after a bind-then-drop probe. That pattern raced parallel Docker
fixtures and failed twice with `address already in use` before Redis started.

Initial fixture startup now retries with a fresh candidate after Docker bind
failure. Restart startup retries the same port, preserving the exact reconnect
contract while allowing prior container mapping cleanup to finish. Retries are
bounded to twenty attempts and retain the final error on exhaustion. The 32-case
TLS credential/trust replacement matrix also uses a four-permit semaphore;
coverage stays parallel without starving container readiness under a full
workspace run.

## Verification

```text
mise exec -- cargo test -p tablerock-engine --test redis_real --locked \
  resubscribes_with_visible_gap_after_redis_restart -- --nocapture
mise exec -- cargo test -p tablerock-engine --test redis_real --locked
mise exec -- cargo clippy -p tablerock-engine --test redis_real --locked -- -D warnings
mise exec -- cargo test --workspace --locked
```

The isolated reconnect case passed. The complete real Redis suite passed 44/44
across the pinned Redis 7.4.9 and 8.8.0 matrix. A fresh full workspace run then
passed with the Redis suite executing under concurrent workspace load. Clippy
passed with warnings denied.
