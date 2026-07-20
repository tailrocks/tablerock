# 555 — Terminal semantic-input priority ingress

Date: 2026-07-21

## Failure evidence

Checks run `29777843667`, Ubuntu job `88471492566`, failed because
`high_rate_mouse_and_resize_do_not_starve_terminal_quit` exceeded its bounded
30-second exit deadline. The same HEAD passed macOS. The event loop throttled
rendering, but keys, parser errors, resize, and pointer traffic still shared one
FIFO; therefore semantic input could remain behind replaceable noise.

## Structural repair

The terminal reader now publishes keys and input errors through a separate
bounded priority lane. The root loop selects that lane first, including while
reducing a bulk burst. Resize and pointer-move samples stay bounded and may be
dropped only when their bulk lane is full; semantic mouse/focus/paste events
still use lossless bounded delivery. This removes the FIFO condition that
allowed the starvation class instead of extending the timeout.

## Verification

```text
for i in 1 2 3 4 5 6 7 8; do
  cargo test -p tablerock-cli --test pty_lifecycle \
    high_rate_mouse_and_resize_do_not_starve_terminal_quit -- --exact
done
# 8/8 passed; each restored the terminal and exited in 1.77–2.21 seconds

cargo test -p tablerock-cli
cargo fmt --all --check
cargo clippy --workspace --all-targets
# passed; existing non-denied warnings unchanged
```

Remote Linux proof: Checks run `29778717500`, Ubuntu job `88474401411`, passed
the full container-free suite at repair commit `a10c434`, including the PTY
regression that failed at the preceding HEAD.

This proof was not stable: later Ubuntu job `88476428834` at `7116097` failed
the same regression after the async decoder never published Quit. Evidence 558
supersedes the reader architecture; lane priority remains a downstream bound,
not the complete repair.

No external product influenced this runtime repair.
