# Phase 2 Safe Diagnostic Evidence

## Checkpoint

This Phase 2 tracer adds the std-only failure vocabulary that future adapters,
the application service, both presentations, telemetry, and the UniFFI bridge
may share. `SafeDiagnostic` carries a failure class, engine, severity, optional
validated engine code and typed position, operator action, observed outcome
certainty, and retry advice.

There is deliberately no arbitrary message field or raw-driver-error
constructor. Driver adapters must map failures into this closed safe contract
before crossing their boundary. Presentation may turn the typed facts into
English guidance without receiving credentials, endpoints, statements, Redis
arguments, or cell values.

## Failure, ambiguity, retry, and redaction

- `SafeCode` has no string variant or parser. PostgreSQL, Redis, and application
  codes are closed enums; ClickHouse codes are numeric `u32` values. Unmapped
  PostgreSQL/Redis failures use `Other` without retaining raw text. Credentials,
  endpoints, usernames, tokens, or driver-message fragments are unrepresentable.
  Engine-specific codes are rejected when attached to a different engine;
  application codes remain engine-neutral.
- `OutcomeCertainty` distinguishes not dispatched, read-only, write not applied,
  write applied, and unknown. Unknown never means failed or safe to retry.
- `RetryAdvice` defaults to `Never`. Automatic retry requires independent
  `OperationSafety::ProvenReadOnly` proof plus not-dispatched or read-only
  certainty. A not-dispatched write is still not automatically retried. Applied,
  not-applied, and unknown writes require user action or an explicit new request.
- Diagnostic positions carry an explicit unit. Server character positions,
  byte offsets, and argument indexes cannot be silently confused.
- Diagnostics and build errors expose only closed enums, numeric ClickHouse
  codes, positions, and counts.

## Deliberate boundary

This is a stable transport-safe taxonomy, not driver mapping. PostgreSQL,
ClickHouse, and Redis adapters still require exhaustive mapping fixtures proving
which official codes map to each closed variant and which severities and
positions are trustworthy. Localized
English presentation text, bounded diagnostic batches, error causes, operation
identity, immediate command rejection, and persistence policy remain later
checkpoints. No raw message fallback is permitted while those are incomplete.

## Evidence

- Public integration tests prove engine and application codes are closed enums
  or numeric values with no arbitrary-text carrier and reject cross-engine code
  attachment.
- Hostile construction tests prove unknown, applied, and even not-dispatched
  write-capable operations cannot claim safe automatic retry.
- Positive tests prove automatic retry remains expressible for proven
  not-dispatched and read-only work.
- Projection tests prove safe code, typed position, operator action, certainty,
  and conservative builder defaults remain observable without a raw message.
- The architecture test includes this module and rejects runtime,
  presentation, driver, network, clock, and secret dependencies.

## Verification record

- `cargo test -p tablerock-core --test diagnostic`: 5 passed.
- `cargo clippy -p tablerock-core --all-targets --locked -- -D warnings`: pass.
- `cargo test --workspace --locked`: 60 passed, 3 ignored.
- Workspace format, clippy, rustdoc, `cargo deny`, `gitleaks`, English-script,
  and complete-diff gates: pass. The already-allowed `hashbrown` duplicate is
  unchanged.

External concepts: closed diagnostic vocabularies, outcome certainty, and fail-closed retry policy
Public sources: <https://doc.rust-lang.org/reference/items/enumerations.html>,
<https://doc.rust-lang.org/std/option/enum.Option.html>, and
<https://doc.rust-lang.org/std/error/trait.Error.html>
TableRock requirements: research 10, 14, 30, 31, and 32
Implementation source: TableRock core architecture and independent tests
Copied code/assets/text: none
