# Phase 1 Fault Restoration Evidence

## Checkpoint

TableRock now proves its returned-error and panic terminal paths in real PTYs.
The production build contains no fault seam. Test builds add one private
post-frame callback that injects a returned input error or panic immediately
after the first complete frame. The callback verifies raw mode is active before
faulting; ordered output proves every writer-backed mode was acquired first.

Returned errors pass through the same explicit restoration function as normal
execution. Panics unwind the session and exercise TermRock's scoped drop
restoration before the exact production panic boundary returns its fixed safe
error. The fault source and panic text never appear in captured terminal output.
No environment variable, command-line switch, feature, or shipped runtime path
can enable fault injection.

## Evidence

- Two parent tests spawn the Rust library test executable inside independent
  80x24 PTYs and select one ignored child fixture by exact test name.
- Each child enters the real TermRock/Crossterm full-screen session, renders one
  TableRock frame, and triggers exactly one controlled fault.
- Both captures prove forward acquisition followed by reverse restoration for
  alternate screen, all five Crossterm mouse modes, bracketed paste, line wrap,
  and cursor state.
- Unix PTYs snapshot termios before spawning, verify raw mode inside the child
  immediately before faulting, then prove exact restoration after child exit.
- Each child completes successfully after asserting the expected safe error
  class through the shared production panic boundary.
- Both captures prove controlled failure text is absent.
- A redirected child remains non-interactive and cannot enter the fault path.
- Normal, signal, pointer-input, returned-error, and panic PTY paths now share
  the same production session ownership and restoration contracts.

Typed engine overflow/resynchronization and the final Phase 1 audit remain.

External concept: scoped terminal restoration under Rust error and unwind only  
Public sources: <https://doc.rust-lang.org/std/panic/fn.catch_unwind.html> and
<https://github.com/tailrocks/termrock/tree/9099b3db0c3318fd183d076c4e8f8002a877be6a>  
TableRock requirement: Roadmap Phase 1 and quality-plan PTY matrix  
Implementation source: TableRock process boundary, TermRock public session API,
portable-pty 0.9 public process API, and independent tests  
Copied code/assets/text: none
