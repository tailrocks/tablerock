# PTY lifecycle scheduler isolation

Date: 2026-07-22

## Failure class

Exact-main CI run `29877301416` passed format, clippy, check, and 578 of 579
container-free tests. `high_rate_mouse_and_resize_do_not_starve_terminal_quit`
then exceeded its unchanged 30-second child-exit bound. The commit changed only
native Swift, generated Xcode metadata, tests, and evidence.

The architecture already routes Ctrl-C through a separate bounded priority
channel and coalesces resize/pointer-move state. Twenty isolated repetitions
passed. The failure occurred only while nextest scheduled PTY process/signal/
terminal-mode timing beside hundreds of unrelated processes.

## Correction

Cargo-nextest now reserves the full global test-thread pool for the
`tablerock-cli::pty_lifecycle` binary. PTY cases therefore run one at a time and
without unrelated nextest process contention. Test input volume, assertions,
30-second bound, and failure behavior remain unchanged; no retry was added.

## Verification

```text
cargo nextest run -p tablerock-cli --test pty_lifecycle \
  -E 'test(high_rate_mouse_and_resize_do_not_starve_terminal_quit)' \
  --stress-count 20
# Summary: 20/20 stress run iterations passed

cargo nextest list --workspace --locked
# configuration accepted; PTY binary selected by exact binary_id override
```

Exact-main hosted proof remains required.

## Documentation source

Cargo-nextest's current primary documentation defines `threads-required` as a
global scheduler limit and supports `"num-test-threads"` for all configured
test threads. This correction uses that documented isolation mechanism.

No external product reference influenced this CI/test correction.
