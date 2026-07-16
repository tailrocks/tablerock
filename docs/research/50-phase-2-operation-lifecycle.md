# Phase 2 Operation Lifecycle Evidence

## Checkpoint

This Phase 2 tracer adds the std-only lifecycle and event-identity vocabulary
for live-session operations. `OperationIdentity` binds an operation and request
to one profile/session/context scope. `OperationPhase` distinguishes queued,
running, streaming, cancel-requested, and terminal states. Terminal
`OperationOutcome` records observed truth instead of treating task abandonment
as server cancellation.

The contract is immutable validation vocabulary. It does not own tasks,
channels, drivers, or mutable engine state. The future application service
remains the sole lifecycle owner and must advance the immutable cursor before
it publishes an `OperationEvent`.

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
  phase-history mismatch, progress regression, and sequence gaps. A gap requires
  resync instead of unbounded buffering or guessed state.
- Progress carries only cumulative row and byte counts and is explicitly
  coalescible. The engine must coalesce before assigning the contiguous delivery
  sequence; replacing pending progress therefore cannot corrupt totals or
  manufacture a delivery gap.
  Phase changes and resync requirements are required delivery. The bounded
  channel and coalescing implementation remains an engine checkpoint.
- Core diagnostics contain IDs, phases, revisions, sequences, and counts only.
  No SQL, Redis arguments, credentials, endpoints, cell bytes, or driver errors
  enter this contract.

## Deliberate boundary

This tracer covers operations that already have a live profile/session/context.
Application-wide profile and connection commands require different scopes and
will be added with the typed command envelope rather than weakening this scope
with optional IDs. Safe error payloads, bounded subscription storage, command
budgets, shutdown, and cross-operation resync snapshots remain required Phase 2
checkpoints.

## Evidence

- Public integration tests prove the queued-to-running-to-streaming vocabulary
  and all four cancellation outcome edges after an observed request. Actual
  driver race ordering remains a real-server/engine checkpoint.
- Hostile transitions prove rejection of skipped phases, false cancellation
  claims, and terminal revival.
- Event tests prove required-versus-coalescible delivery classification,
  cumulative progress, duplicate delivery rejection, sequence-gap resync,
  foreign operation rejection, phase-history continuity, progress monotonicity,
  and constructor rejection of illegal phase edges.
- The architecture test includes this module and rejects runtime,
  presentation, driver, network, clock, and secret dependencies.

## Verification record

- `cargo test -p tablerock-core --test operation`: 4 passed.
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
