# Phase 2 Value and Capability Contract Evidence

## Checkpoint

The second Phase 2 tracer adds a std-only vocabulary for engine capability facts
and owned values. `Engine` names exactly PostgreSQL, ClickHouse, and Redis.
`CapabilityFact` binds one capability to one engine. A revisioned, fixed-width
`CapabilitySnapshot` gives every known capability an unassessed, supported, or
explicitly unsupported state, so omission cannot masquerade as unsupported.
Unsupported reasons keep
not-applicable behavior, server-version limits, permissions, driver gaps,
deployment limits, and protocol semantics distinct. No shared capability
pretends that unlike engine behavior is equivalent.

`OwnedValue` is opaque and can only be created through validating constructors;
callers inspect it through a borrowed `ValueRef`. It distinguishes null,
boolean, signed and unsigned integers, exact float bits, exact decimal text,
text, binary, invalid recognized values, and unknown engine values. Text
and binary remain different even when empty. Unknown values retain a bounded
engine type name and bounded raw payload. Text, binary, invalid, and unknown
payloads carry an explicit complete or truncated state; a known original length
must be larger than its stored prefix.

## Bounds, failure, and redaction

- `ByteLimit` is supplied by the owning operation. This checkpoint does not
  invent a global cell size before Phase 2 driver measurements establish the
  page and process-memory budgets required by fixed decision 31.
- Slice constructors check byte length before copying, so rejected input causes
  no core allocation. Ownership constructors validate without copying and
  return the original `Vec<u8>` or `String` on failure.
- Bounds count encoded bytes, not Unicode scalar values. This matches page arena
  accounting and prevents a multibyte string from bypassing a byte budget.
- Safe errors contain only actual and allowed byte counts. Custom `Debug`
  implementations expose safe kind, length, engine where present on metadata,
  and truncation facts but never cell bytes, cell text, decimal text, or engine
  type names. The content-bearing borrowed projection intentionally has no
  `Debug` implementation.
- Unknown values are inspectable through explicit accessors but are not silently
  promoted to an editable normalized type. Editability remains a later typed
  plan decision.
- No cancellation or partial-result claim is made by this value-only tracer.

## Evidence

- Contract tests prove fixed snapshot coverage, unassessed state, explicit
  per-engine support and unsupported reasons, revision identity, and rejection
  of cross-engine facts.
- Boundary tests cover exact limits, one-byte overflow, empty values, UTF-8 byte
  accounting, ownership recovery after rejection, null/empty/whitespace,
  integer zero/false/text/binary distinction, signed float zero bit
  preservation, exact decimal text, truncation validation, invalid values, and
  non-empty unknown ClickHouse type retention.
- Redaction tests prove cell text and engine type names are absent from debug
  output.
- The architecture test includes the new module and continues to reject runtime,
  presentation, database-client, network, and clock dependencies.
- There is no compatibility shim. This is the first authoritative value and
  capability vocabulary and may be forward-refactored before public release.
- Temporal, UUID, JSON, array, map, tuple, variant projections and
  engine-specialized editable representations remain blocked on the real-driver
  value corpus. Until then they use explicit unknown or invalid values; this
  checkpoint makes no full PostgreSQL or ClickHouse type-coverage claim.

## Verification record

- `cargo test -p tablerock-core`: 11 passed.
- `cargo clippy -p tablerock-core --all-targets --all-features --locked -- -D
  warnings`: pass.
- `cargo doc -p tablerock-core --no-deps --locked`: pass.
- `cargo test --workspace --locked`: 45 passed, 3 ignored PTY child fixtures
  executed by their parent harnesses.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D
  warnings` and `cargo doc --workspace --no-deps --locked`: pass.
- `cargo deny check`: advisories, bans, licenses, and sources pass; the known
  Ratatui graph retains two allowed transitive `hashbrown` versions.
- `gitleaks detect --source . --no-banner --redact`, English-script scan,
  `git diff --check`, architecture review, specification review, and complete
  diff review: pass after correcting review findings around constructible
  invalid truncation, missing invalid values, incomplete capability state,
  empty engine type identity, and test/evidence coverage.

External concepts: owned byte buffers, exact floating-point bit representation,
and explicit tagged unions only  
Public sources: <https://doc.rust-lang.org/std/vec/struct.Vec.html>,
<https://doc.rust-lang.org/std/string/struct.String.html>, and
<https://doc.rust-lang.org/std/primitive.f64.html>  
TableRock requirements: research 03, 10, 14, 30, and fixed decision 31  
Implementation source: TableRock core architecture and independent tests  
Copied code/assets/text: none
