# Plan 015 — Redis namespace projection, INFO snapshot, sequential apply

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| UI `redis_namespace` projection (`:` path, binary flat group) | done |
| Static policy: no `cmd("KEYS")` in redis driver | done |
| `server_info_snapshot` bounded INFO fields + sample time | done |
| `apply_authorized_mutation` SET/DEL sequential (non-transactional) | done |
| Type views / command editor / full edit suite | open |

## Verification

```text
cargo test -p tablerock-tui --lib redis_namespace
cargo test -p tablerock-engine --lib scan_policy
cargo test -p tablerock-engine --lib
```
