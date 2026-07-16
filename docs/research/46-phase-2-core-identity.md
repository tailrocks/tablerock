# Phase 2 Core Identity Evidence

## Checkpoint

The first Phase 2 tracer creates `tablerock-core` as the std-only stable language
shared by future engine and client crates. It defines distinct owned IDs for
profiles, sessions, contexts, tabs, queries, results, rows, mutations,
operations, and requests. Internally every ID is one nonzero 128-bit value;
zero is rejected rather than becoming an uninitialized sentinel. Its public
canonical representation is an FFI-safe `IdParts { high: u64, low: u64 }`, a
big-endian 16-byte array, or exactly 32 hexadecimal digits. All three forms
round-trip, accept upper/lowercase input where applicable, and emit one
lowercase canonical form. Identical payloads across ID kinds remain
non-interchangeable Rust types.

Core does not generate IDs. Generation belongs to an engine application-service
identity factory so future randomness/platform policy cannot introduce a hidden
RNG or clock into stable contracts, and database drivers cannot mint aggregate
identity. Core accepts already-generated canonical parts/bytes/text. Debug
includes only the safe ID kind and canonical value.

`Revision` and `EventSequence` are distinct checked monotonic `u64` counters.
Both start at zero, advance explicitly, and return neutral `CounterOverflow` at
the numeric boundary. Revision candidates classify as stale/current/future;
event candidates classify as stale-or-duplicate/next/gap. Wrapping can never
make a stale aggregate/event appear new. Arbitrary numeric construction is named
`from_wire_u64` to make trusted decode/restoration validation explicit.

## Bounds, failure, and compatibility

- IDs and `IdParts` occupy 16 bytes and contain no string allocation, borrow,
  client row, secret, or driver type. Canonical decode rejects zero, malformed
  length, and invalid hexadecimal with a safe byte position.
- Revision advancement is constant-memory and fallible at exhaustion.
- ID uniqueness generation is not claimed yet. It is an engine-owned blocking
  checkpoint because core deliberately has no RNG/clock. Schema-version and
  UniFFI derivation enter with their owning command/event/facade gates.
- There is no backward-compatibility shim. These are the first authoritative
  types and may be forward-refactored before public contract release.

## Evidence

- Public tests exercise every ID kind and prove parts/bytes/text round trips,
  fixed 16-byte size, canonical case, zero rejection, malformed length, invalid
  hex position, independent known big-endian vectors, and minimum/maximum
  boundary values.
- Counter tests prove initial/next behavior, overflow rejection, stale revision,
  replay/duplicate event, next event, and sequence gap classification.
- An architecture test proves the crate has no dependency section and rejects
  runtime, presentation, driver, network, and clock vocabulary in this tracer.
- Workspace policy continues to forbid unsafe code.

## Verification record

- Evidence platform: Apple arm64, macOS `26.5.2` build `25F84`; Rust/Cargo
  `1.97.0`; workspace minimum Rust `1.95`.
- `cargo test --workspace --locked`: 40 passed, 3 ignored PTY child fixtures
  executed by their parent tests.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D
  warnings`: pass.
- `cargo doc --workspace --no-deps --locked`: pass.
- `cargo deny check`: advisories, bans, licenses, and sources pass; the known
  Ratatui graph contains two allowed transitive `hashbrown` versions.
- `gitleaks detect --source . --no-banner --redact`: pass, no leaks.
- `git diff --check`, complete-diff review, forbidden dependency/architecture
  review, and English-script scan: pass; only this checkpoint's intended files
  changed.
- Dependency/license result: `tablerock-core` adds no dependency; workspace
  remains Apache-2.0. No server, database value, secret, cancellation, or
  partial outcome exists in this identity-only checkpoint.

External concept: nonzero integer newtypes and checked arithmetic only  
Public source: <https://doc.rust-lang.org/std/num/struct.NonZeroU128.html>  
TableRock requirement: Roadmap Phase 2 stable opaque IDs and aggregate revisions  
Implementation source: TableRock core architecture and independent tests  
Copied code/assets/text: none
