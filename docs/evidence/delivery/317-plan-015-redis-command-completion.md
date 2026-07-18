# Plan 015 residual — Redis command completion (curated metadata)

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| License/provenance decision for command metadata | done (curated names only) |
| Expanded `ALL_COMMANDS` completion table | done |
| `build_redis_session` pure builder | done |
| Workbench `open_completion` branches on Redis | done |
| Commit uses catalog_revision=0 on Redis | done |
| Unit: prefix HGET, commit + space, no SQL keywords | done |

## Decision

Fixed decision calls for “official command metadata.” Full redis-doc JSON
vendoring was not adopted (license/provenance STOP avoided by choosing a
hand-curated open-command *name* table already used for safety
classification). Completion candidates are names + safety kind labels
(`command/read-only`, `command/may-write`, `command/blocking-denied`).
Not a third-party dump; expand the table as product coverage grows.

## Evidence

```text
cargo test -p tablerock-tui --lib redis_
```

## Remaining work

- Disposable-connection isolation for intentional blocking ops
- Pub/Sub UI (post-parity)
