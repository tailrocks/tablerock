# Phase 2 Profile Property Policy Evidence

## Checkpoint

This Phase 2 tracer defines the versioned, bounded property/value-source model
used by later saved and temporary profiles. It makes one binary policy decision:
passwords, TLS client private keys, and TLS private-key passwords cannot use the
ordinary literal constructor. Those properties require a Rust-owned
`SecretSource`. Usernames, endpoints, contexts, server names, CA certificates,
and client certificates are not secret material and may be literal or sourced.

All ten properties may use `SecretSource`. This preserves metadata-only
1Password mapping for host, port, database/index, username, password, and TLS
fields instead of incorrectly treating references as password-only.

## Failure, safety, redaction, and bounds

- `ProfilePropertySet` schema version 1 rejects unknown versions, duplicate
  properties, and more bindings than the closed property vocabulary.
- Literal host and TLS server names are bounded to 253 bytes; ports to five
  ASCII decimal bytes and `1..=65535`; default contexts and usernames to 128
  bytes; CA and client certificates to 64 KiB.
- Empty literals fail. Password/private-key material has a zero literal limit
  and fails through `LiteralForbidden` before any profile can be constructed.
- Property bindings and sets are intentionally non-cloneable because they may
  own a non-cloneable `SecretSource`.
- `Debug` exposes property names and source categories only. It omits endpoint,
  username, certificate, environment, reference, and plaintext contents.
- Errors contain property categories and lengths only; no supplied value is
  retained or rendered.

## Deliberate boundary

This is the property-policy layer, not the complete profile aggregate. It does
not yet define profile name/organization, engine applicability, TLS policy,
safety mode, timeouts, required-field readiness, persistence encoding,
migration, resolution, URL import, or either UI projection. The
[profile connection snapshot checkpoint 55](55-phase-2-profile-snapshot.md) composes these
bindings with stable profile identity and engine readiness. The Turso actor
follows only after sequential migration semantics are proven.

No I/O, driver, runtime, environment lookup, 1Password process, Keychain call,
resolved secret, persistence, logging, telemetry, or FFI value exists here.

## Evidence

- Public tests map a real structured 1Password fixture across every property
  and cover both source categories.
- Hostile tests reject literal secret material, empty/oversized values,
  signed, malformed, and out-of-range ports, duplicates, and unknown schema
  versions; port boundaries `1` and `65535` pass.
- Redaction fixtures prove literal and environment contents do not appear in
  binding or set diagnostics.
- The core architecture test includes the profile module and continues to ban
  runtime, presentation, driver, network, and clock dependencies.

## Verification record

- `cargo test -p tablerock-core --test profile`: 5 passed.
- `cargo clippy -p tablerock-core --all-targets --locked -- -D warnings`: pass.
- `cargo test --workspace --locked`: 74 passed, 3 ignored.
- Workspace format, Clippy, rustdoc, `cargo deny`, `gitleaks`, English-script,
  and complete-diff gates: pass. The already-allowed `hashbrown` duplicate is
  unchanged.

External concepts: typed property/value-source bindings and closed schema tags
Public sources: none required; policy derives from approved TableRock research
TableRock requirements: research 02, 06, 10, 14, 30, 31, and 32
Implementation source: TableRock core architecture and independent tests
Copied code/assets/text: none
