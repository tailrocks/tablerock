# Evidence 653: concurrent SSH fixture identity

## Claim

Real SSH tests no longer derive Docker names from process ID plus wall-clock
time alone. Concurrent tests observed identical timestamps and attempted to
create the same PostgreSQL container, producing deterministic Docker `409
Conflict` failures under a full workspace run.

Fixture identity now includes a process-local atomic nonce before the timestamp.
Every concurrent caller receives a distinct name even on clocks with coarse or
repeated readings. Ordering is irrelevant; relaxed atomic ordering is enough
for uniqueness.

## Verification

```text
mise exec -- cargo test -p tablerock-engine --test ssh_tunnel_real --locked
mise exec -- cargo clippy -p tablerock-engine --test ssh_tunnel_real --locked -- -D warnings
mise exec -- cargo test --workspace --locked
```

All eight real SSH tests passed concurrently. The fresh full workspace run then
passed with both SSH and bounded Redis Docker matrices active. Clippy passed
with warnings denied.
