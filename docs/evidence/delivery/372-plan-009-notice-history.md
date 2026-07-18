# Plan 009 residual — NOTICE history panel

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| Per-tab `notice_history` ring (cap 16) | done |
| `push_notice` on GridStreamComplete | done |
| Status line still shows latest notice | done |
| ShowNotices → inspector kind `notices` | done |
| ClearNotices | done |
| Unit tests (ring + complete path) | done |

## Decision

Status bar remains the live surface for the latest NOTICE. History is a
bounded per-tab ring (oldest dropped) so multi-statement sessions keep
prior RAISE NOTICE/DETAIL lines without unbounded growth. Text stays the
redacted severity+message (+detail/hint) already produced by the engine
drain; no SQL or cell values.

## Evidence

```text
cargo test -p tablerock-tui --lib notice_history
cargo test -p tablerock-tui --lib grid_stream_complete
cargo test -p tablerock-tui --lib
```

## Remaining work

- Per-statement ordinal association (optional)
