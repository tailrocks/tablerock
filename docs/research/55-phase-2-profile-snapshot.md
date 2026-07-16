# Phase 2 Profile Connection Snapshot Evidence

## Checkpoint

This Phase 2 tracer composes stable `ProfileId`, monotonic `Revision`, engine,
bounded redacted name, property bindings, TLS policy, Rust-owned safety selection,
and finite connection/operation/result limits into immutable schema-version-1
`ProfileConnectionSnapshot` values.

Fixed decision 31 permits only `ReadOnly` and `ConfirmWrites`; the stale
`Unrestricted` option in research 02 is removed in this checkpoint. Specific
destructive confirmation remains separately mandatory below presentation.

## Readiness and TLS invariants

- PostgreSQL, ClickHouse, and Redis snapshots all require host and port
  bindings. Either may be an unresolved `SecretSource`; construction checks
  stable structure and never resolves it.
- TLS is one closed policy: disabled, verified system roots, verified custom CA,
  or dangerous invalid-certificate acceptance with typed `LocalTestingOnly`
  acknowledgement.
- Disabled TLS rejects every TLS property. Custom-CA verification requires a CA
  property. System-root and dangerous-bypass modes reject an irrelevant custom
  CA property.
- Client certificate and private key are an atomic pair. A private-key password
  requires that pair. Private keys and their passwords remain non-literal by
  the property-policy checkpoint.
- Names are 1..=128 UTF-8 bytes, reject all-whitespace/control content, and are
  omitted from `Debug` together with every property value.

## Finite owner limits

Every value must be nonzero. Owner maxima are 120 seconds to connect, one hour
per operation, 1,000,000 result rows, and 1 GiB of result bytes. Initial product
defaults used by fixtures are 10 seconds, 30 seconds, 10,000 rows, and 64 MiB.
These are profile limits, not permission to allocate or fetch them in one batch;
page and command owners retain their tighter independent bounds.

## Deliberate boundary

This checkpoint defines a connect-ready settings sub-snapshot, not the complete
durable profile aggregate. Organization, tags, favorites, ordering,
preferences, saved/temporary lifecycles, stale-revision mutation commands,
engine adapter resolution, URL import, migration encoding, and UI projections
remain downstream work. `ProfileSafetyMode` stores a closed Rust-owned
selection; command classification, write rejection, review tokens, and
destructive confirmation enforcement remain mandatory below presentation.

No environment lookup, 1Password process, Keychain call, resolved secret,
driver, runtime, persistence, logging, telemetry, or FFI value is introduced.
The [profile aggregate checkpoint 56](56-phase-2-profile-aggregate.md) composes
this sub-snapshot with durability, organization, preferences, and revision
replacement. Sequential Turso migration compatibility follows.

## Evidence

- Public tests construct redacted snapshots for all three engines.
- A full engine/property matrix rejects missing host or port and accepts an
  unresolved source without resolving it.
- TLS tests reject disabled-property contradictions, missing custom roots,
  irrelevant roots, incomplete client identity, and orphan key passwords.
- Name, schema-version, timeout, row, and byte boundary tests fail closed.
- Snapshot diagnostics prove name, host, and port contents absent.

## Verification record

- `cargo test -p tablerock-core --test profile_snapshot`: 5 passed.
- `cargo clippy -p tablerock-core --all-targets --locked -- -D warnings`: pass.
- `cargo test --workspace --locked`: 79 passed, 3 ignored.
- Workspace format, Clippy, rustdoc, `cargo deny`, `gitleaks`, English-script,
  and complete-diff gates: pass. The already-allowed `hashbrown` duplicate is
  unchanged.

External concepts: immutable versioned aggregate, explicit TLS state machine,
and independently bounded resource owners
Public sources: none required; policy derives from approved TableRock research
TableRock requirements: research 02, 06, 10, 14, 30, 31, and 32
Implementation source: TableRock core architecture and independent tests
Copied code/assets/text: none
