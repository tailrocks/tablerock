# 547 — PTY terminal-burst fairness

Date: 2026-07-21

## Failure

Repeated Ubuntu CI runs timed out
`high_rate_mouse_and_resize_do_not_starve_terminal_quit`. The runtime reduced
one backend event and immediately painted one full frame. A finite resize and
pointer-motion burst could therefore place dozens of renders ahead of an
already queued quit key.

## Structural repair

The CLI now drains a bounded ready terminal burst through the TEA reducer before
painting once. Exit effects remain immediate. The 64-event cap returns control
to signal and engine ingress selection.

Painted geometry remains authoritative: pointer motion is coalesced while a
frame is dirty, and button/scroll events are retained until the new frame has
published its geometry. Thus burst reduction cannot map a click against stale
layout.

## Verification

```text
cargo test -p tablerock-cli --test pty_lifecycle
cargo test -p tablerock-cli --test pty_lifecycle \
  high_rate_mouse_and_resize_do_not_starve_terminal_quit  # 10 consecutive passes
```

The starvation case completed in 1.63–7.81 seconds across the ten local stress
trials, below its closed 30-second bound. The complete PTY lifecycle suite also
passes, including resized painted-geometry mouse behavior and exact terminal
restoration.

This repairs failures observed in GitHub Actions runs `29773657777` and
`29774248346`; the pushed checkpoint must make the Ubuntu gate authoritative.

No external product influenced this runtime scheduling repair.
