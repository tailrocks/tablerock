# Phase 2 Saved Profile Create Evidence

## Checkpoint

Sequential persistence migration `0003-saved-profiles.sql` adds normalized
saved-profile, ordered-tag, and ordered-property tables. Its separate migration
document records the before/after contract and `MIGRATING.md` links it after
`0001` and `0002`.

`PersistenceActor::create_profile` accepts only core-issued
`PersistableProfile<'_>`. That token can be obtained only from an aggregate
whose durability is `Saved`; temporary aggregates remain structurally unable
to call the persistence command. Encoding occurs before queue submission into
a bounded owned command, then one worker-owned transaction inserts the parent,
tags, and properties.

## Stored contract

- Opaque profile IDs use exact 16-byte big-endian storage.
- Revisions use exact 8-byte big-endian unsigned storage, avoiding SQLite's
  signed-integer narrowing.
- Aggregate, connection, property, and secret-source schema versions are
  explicit columns.
- Engine, TLS, safety, finite limits, organization, preferences, property
  identity, source kind, and ordered tags/properties remain separate facts.
- Literal, 1Password, prompt, environment, Keychain, and explicitly dangerous
  local plaintext source forms each have a closed numeric discriminator and
  source-specific columns. No source is resolved during persistence.
- Database constraints bound IDs, revisions, labels, limits, cardinality,
  discriminators, source schema, and variant payload shapes. Core constructors
  remain the authoritative semantic validator.
- Organization indexes support future bounded favorite/order/name and tag
  filtering without full-table application scans.

Dangerous plaintext and Keychain command copies use a zeroizing owned buffer.
The explicit dangerous-local source is persisted because that is its reviewed
contract; ordinary password/private-key literals remain impossible in core.
Public errors remain metadata-only and never include profile names, property
values, references, or database messages.

## Failure and atomicity

Duplicate stable IDs return `ProfileAlreadyExists` before mutation. A test-only
database trigger aborts an ordered-tag insert after the parent insert. The
actor reports `ProfileWrite`, the transaction rolls back, and independent
inspection proves both parent and property tables remain empty. Queue and
operation deadlines retain the actor's existing bounds.

## Evidence

- PostgreSQL, ClickHouse, and Redis saved aggregates are created through the
  same actor contract.
- Fixtures cover every supported property source kind, ordered tags,
  organization, preferences, TLS client pairing, safety, and finite limits.
- Independent Turso inspection proves 3 parents, 6 tags, 27 properties, and the
  expected dangerous-source payload lengths without reading payload content.
- Duplicate insertion is rejected and reopen reports exact schema version 3.
- Injected child-row failure proves whole-aggregate rollback.

## Deliberate boundary

This checkpoint implements create, not full CRUD. Validated single-profile read
is now implemented by the subsequent checkpoint in
[`65-phase-2-saved-profile-read.md`](65-phase-2-saved-profile-read.md). Bounded
list/filter projections, revision-CAS replacement, deletion policy,
dangerous-source user warnings, and crash/disk/permission injection during
profile writes remain required. No UI or resolver consumes persisted profiles
yet.

## Verification record

- `cargo test -p tablerock-persistence`: 10 passed across 6 suites.
- `cargo clippy -p tablerock-persistence --all-targets --locked -- -D warnings`:
  pass.
- `cargo test --workspace --all-targets --locked`: 94 passed, 3 ignored.
- Workspace format, Clippy, rustdoc, leak scan, and diff checks: pass.

External concepts: normalized aggregate persistence and transactional parent/child insertion
Public sources: no new external source; schema derives from approved TableRock core contracts
Implementation source: TableRock-owned migration, encoder, actor command, and real-file tests
Copied code/assets/text: none
