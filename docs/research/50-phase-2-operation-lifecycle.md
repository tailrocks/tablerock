# Phase 2 Operation Lifecycle Evidence

## Checkpoint

This Phase 2 tracer adds the std-only lifecycle and event-identity vocabulary.
`OperationIdentity` binds an operation and request to one typed application,
profile, session, or context command scope. `OperationPhase` distinguishes queued,
running, streaming, cancel-requested, and terminal states. Terminal
`OperationOutcome` records observed truth instead of treating task abandonment
as server cancellation.

The contract does not own tasks, drivers, or mutable engine state. It now owns
the bounded per-operation delivery queue and immutable consumer cursor. The
future application service remains the sole lifecycle owner and must publish
through that queue.

## Failure, cancellation, bounds, and redaction

- Legal edges are explicit. Terminal states cannot revive, queued work cannot
  skip directly to streaming, and server-confirmed/client-stopped/completed-
  before-cancel outcomes require an observed `CancelRequested` state.
- Cancellation outcomes distinguish local client stop, server confirmation,
  completion before cancellation, and unknown outcome. Reconnect may not infer
  success or retry an ambiguous write from any of these facts.
- Every event carries operation/request/profile/session/context identity,
  aggregate revision, and event sequence. An immutable `OperationCursor` retains
  the accepted phase, revision, sequence, and cumulative progress. It rejects
  foreign identity, stale or duplicate delivery, revision mismatch,
  phase-history mismatch, progress regression, and required-event sequence
  gaps. A cumulative progress event may cross a gap only when the queue marks
  the exact last sequence preceding its coalesced range.
- Progress carries only cumulative row and byte counts and is explicitly
  coalescible. `OperationEventQueue` replaces only consecutive pending progress,
  has a nonzero capacity capped at 4,096, and never drops a required event
  silently. Capacity exhaustion or producer sequence loss replaces pending
  delivery with one required resync marker carrying the last delivered cursor.
- Core diagnostics contain IDs, phases, revisions, sequences, and counts only.
  No SQL, Redis arguments, credentials, endpoints, cell bytes, or driver errors
  enter this contract.

## Deliberate boundary

Identity reuses `CommandScope`; it does not add optional IDs or a second scope
hierarchy. Safe error payloads, multi-subscription ownership, command budgets,
shutdown, and cross-operation resync snapshots remain required Phase 2
checkpoints.

## Evidence

- Public integration tests prove the queued-to-running-to-streaming vocabulary
  and all four cancellation outcome edges after an observed request. Actual
  driver race ordering remains a real-server/engine checkpoint.
- Hostile transitions prove rejection of skipped phases, false cancellation
  claims, and terminal revival.
- Event tests prove required-versus-coalescible delivery classification,
  cumulative progress gaps, bounded consecutive coalescing, overflow/gap
  resync, invalid capacity, duplicate/foreign rejection, phase-history
  continuity, progress monotonicity, and illegal-edge rejection.
- The architecture test includes this module and rejects runtime,
  presentation, driver, network, clock, and secret dependencies.

## Verification record

- `cargo test -p tablerock-core --test operation`: 8 passed.
- `cargo clippy -p tablerock-core --all-targets --locked -- -D warnings`: pass.
- `cargo test --workspace --locked`: 55 passed, 3 ignored.
- Workspace format, clippy, rustdoc, `cargo deny`, `gitleaks`, English-script,
  and complete-diff gates: pass. The already-allowed `hashbrown` duplicate is
  unchanged.

External concepts: finite-state transition validation and monotonic event cursors
Public sources: <https://doc.rust-lang.org/reference/items/enumerations.html>,
<https://doc.rust-lang.org/std/primitive.u64.html>, and
<https://doc.rust-lang.org/std/error/trait.Error.html>
TableRock requirements: research 10, 14, 30, 31, and 32
Implementation source: TableRock core architecture and independent tests
Copied code/assets/text: none
