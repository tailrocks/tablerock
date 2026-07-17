# Phase 2 Secret Source Evidence

## Checkpoint

This Phase 2 tracer adds the versioned dependency-minimal `SecretSource` reference model
that stable profiles will use. The closed variants are account-pinned
1Password, prompt on connect, host environment, native Keychain reference, and
explicitly acknowledged dangerous plaintext for local testing.

Current official 1Password documentation defines secret references as
`op://vault/item/[section/]field`, permits names or unique identifiers, calls
IDs the most stable choice, and states that object IDs contain 26 letters and
numbers. The model therefore stores pinned 26-character account/vault/item IDs,
bounded optional section and field ID segments, and a bounded display
breadcrumb. It does not persist a name-based URI or a resolved value.

## Failure, safety, redaction, and bounds

- `SecretSource` schema version 1 rejects unknown wire versions.
- 1Password object IDs are exactly 26 ASCII letters/numbers. Section/field ID
  segments are nonempty, bounded to 128 ASCII identifier bytes, and cannot
  contain URI separators, queries, fragments, escapes, whitespace, or controls.
  Display breadcrumbs are nonempty and bounded to 256 UTF-8 bytes.
- Host environment names use a bounded 128-byte identifier grammar and carry a
  variable name only, never its resolved value.
- Keychain persistent references are nonempty and bounded to 4 KiB. They are
  opaque bytes resolved only by the future thin native adapter.
- Prompt-on-connect contains no durable value.
- Dangerous plaintext is the sole stable secret-byte exception, bounded to 64
  KiB and constructible only with the typed `LocalTestingOnly`
  acknowledgement. Its persistence risk remains explicit after save. It is not
  cloneable, supports explicit early clearing, and uses `zeroize` 1.9.0 on
  drop. That dedicated crate guarantees compiler-resistant zeroing and clears
  the entire `Vec` capacity; TableRock keeps `unsafe` forbidden.
- `Debug` for every reference and source omits IDs, segments, breadcrumbs,
  environment names, Keychain bytes, and plaintext bytes. Errors expose only
  enum fields, lengths, and byte indexes.

## Deliberate boundary

This checkpoint defines source references, not resolution or the complete
profile schema. No `op` process, environment access, Keychain call, persistence,
logging, telemetry, FFI event, or resolved-secret buffer exists here. The engine
must later prove bounded account-pinned `op read`, requested-field-only
resolution, transient buffer zeroization, prompt lifetime, Keychain ownership,
and the absence of resolved values from stable state. Profile properties will
next forbid ordinary literal values for credential fields.

Dangerous plaintext persistence warning UX and clean-machine storage inspection
remain gates before that variant is wired into Turso or either presentation.
This contract does not claim persistence safety merely because memory ownership,
bounds, clearing, and diagnostic redaction are enforced.

## Evidence

- Public integration tests prove account/vault/item IDs, field segments,
  breadcrumbs, environment names, Keychain references, plaintext bounds, source
  versioning, persistence-risk classification, and native-adapter classification.
- Hostile tests reject wrong-length object IDs, URI separators in reference
  segments, invalid environment names, and unknown schema versions.
- Redaction tests directly format every inner reference type with real fixture
  values and prove none appear. Plaintext tests prove explicit early clear
  empties the owner; the pinned `zeroize` contract covers the complete backing
  allocation capacity.
- The architecture test includes this module and rejects runtime,
  presentation, driver, network, clock, and secret-resolution dependencies.

## Verification record

- `cargo test -p tablerock-core --test secret`: 4 passed.
- `cargo clippy -p tablerock-core --all-targets --locked -- -D warnings`: pass.
- `cargo test --workspace --locked`: 69 passed, 3 ignored.
- Workspace format, clippy, rustdoc, `cargo deny`, `gitleaks`, English-script,
  and complete-diff gates: pass. The already-allowed `hashbrown` duplicate is
  unchanged.

External concepts: stable opaque references, tagged source unions, and explicit dangerous-local acknowledgement
Public sources: <https://www.1password.dev/cli/secret-references>,
<https://www.1password.dev/cli/reference>, and
<https://docs.rs/zeroize/1.9.0/zeroize/>
TableRock requirements: research 02, 10, 14, 30, 31, and 32
Implementation source: TableRock core architecture and independent tests
Copied code/assets/text: none
