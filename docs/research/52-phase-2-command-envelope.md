# Phase 2 Typed Command Envelope Evidence

## Checkpoint

This Phase 2 tracer adds the std-only submission vocabulary for the first safe
application-service commands. `CommandEnvelope` carries a schema version,
request ID, exactly one typed aggregate scope, expected revision, an
owner-validated finite budget, optional parent operation, and typed intent.

The initial intents are `TestProfile`, `Connect`, `Disconnect`,
`RefreshCatalog`, `FetchPage`, `Cancel`, and `Shutdown`. Execution, mutation,
profile persistence, secret resolution, export/import, and administration are
not represented until their bounded argument and reviewed-safety contracts
exist. This prevents temporary string commands or bypass paths.

## Scope, safety, redaction, cancellation, and bounds

- `CommandScope` is an enum: application, profile, session, or full
  profile/session/context. It is not a struct of optional IDs. Each intent has
  exactly one permitted scope shape; mismatches fail before submission.
- Every command carries an expected aggregate revision. The future engine must
  reject stale/future expectations against its authoritative snapshot rather
  than silently applying intent to different state.
- Safety and redaction are derived from `CommandIntent`; callers cannot label a
  command themselves. Current intents are read-only or lifecycle and contain
  metadata only. No statement, Redis arguments, credentials, endpoints, cell
  values, or arbitrary message text can enter the envelope.
- `CommandBudget` requires nonzero duration, event-count, response-byte, and
  page-row values, then validates them against caller-owned
  `CommandBudgetLimits`. Only the non-constructible validated token enters an
  envelope. Relative milliseconds keep core independent of clocks.
- `PageRequest` owns result ID/revision and a checked global row range. Zero or
  overflowing ranges are unrepresentable; requested rows must also fit the
  validated command budget.
- Optional parent operation identity establishes cancellation ancestry without
  claiming driver/server cancellation. `Cancel` targets a typed operation ID;
  observed outcome remains governed by the operation lifecycle contract.
- Required events cannot be silently discarded when the event budget is
  reached. The future engine queue must coalesce cumulative progress first and
  emit explicit resync/terminal state according to the operation contract.

## Failure and deliberate boundary

Construction rejects zero/excessive budgets, unsupported schema versions,
scope mismatch, zero/overflowing page ranges, and page requests beyond their
row budget. Errors contain only enum fields and numeric limits.

This checkpoint validates shape, not current authority. Session existence,
expected-revision comparison, parent/target scope ownership, deadline passage,
queue capacity, byte accounting, immediate rejection as `SafeDiagnostic`, and
shutdown execution belong to the application service. Statement execution and
writes remain absent until parser-derived safety and review tokens make unsafe
states unrepresentable.

## Evidence

- Public integration tests prove every budget field is nonzero and every owner
  ceiling rejects its hostile boundary.
- Scope-matrix tests prove each initial intent's sole valid aggregate scope and
  representative cross-scope rejection.
- Envelope tests prove schema-version rejection, expected revision retention,
  derived read-only/lifecycle safety, metadata-only redaction, and parent
  operation retention.
- Page tests prove zero/range-overflow rejection and enforcement of the
  validated row budget.
- The architecture test includes this module and rejects runtime,
  presentation, driver, network, clock, and secret dependencies.

## Verification record

- `cargo test -p tablerock-core --test command`: 5 passed.
- `cargo clippy -p tablerock-core --all-targets --locked -- -D warnings`: pass.
- `cargo test --workspace --locked`: 65 passed, 3 ignored.
- Workspace format, clippy, rustdoc, `cargo deny`, `gitleaks`, English-script,
  and complete-diff gates: pass. The already-allowed `hashbrown` duplicate is
  unchanged.

External concepts: versioned envelopes, tagged scopes, finite relative budgets, and checked ranges
Public sources: <https://doc.rust-lang.org/reference/items/enumerations.html>,
<https://doc.rust-lang.org/std/primitive.u64.html>, and
<https://doc.rust-lang.org/std/option/enum.Option.html>
TableRock requirements: research 10, 14, 30, 31, and 32
Implementation source: TableRock core architecture and independent tests
Copied code/assets/text: none
