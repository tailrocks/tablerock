# 558 — Dedicated terminal decoder thread

Date: 2026-07-21

## Contradicting evidence

Checks run `29779319986`, Ubuntu job `88476428834`, again timed out
`high_rate_mouse_and_resize_do_not_starve_terminal_quit` after 30 seconds.
Priority queues from evidence 555 could prioritize only events already decoded;
the async Crossterm `EventStream` decoder itself shared TableRock's
single-threaded TEA runtime and did not publish the Ctrl-C under the Linux PTY
resize storm. One earlier green run was therefore insufficient evidence.

## Structural repair

Blocking Crossterm decoding now owns a named OS thread. The TEA runtime only
consumes bounded decoded events. Ctrl-C and decoder errors enter a dedicated
bounded priority lane; ordinary semantic events preserve FIFO in a lossless
bounded lane; only saturated resize and pointer-move samples may be discarded.
The reader observes channel closure after any completed read and otherwise ends
with the process, so it cannot hold Tokio runtime shutdown open.

This removes the enabling scheduling dependency: terminal byte decoding no
longer competes with update/effect/render work on the current-thread runtime.

## Verification

```text
for i in 1 2 3 4 5 6 7 8; do
  cargo test -p tablerock-cli --test pty_lifecycle
done
# 32/32 lifecycle cases passed; each exact terminal restoration assertion held

cargo test -p tablerock-cli
cargo fmt --all --check
cargo clippy -p tablerock-cli --all-targets
# passed; existing non-denied warnings unchanged
```

Fresh repeated Linux CI proof remains required before preview publication.

No external product influenced this runtime repair.
