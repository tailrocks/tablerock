# Phase 2 Saved Profile Read Evidence

## Checkpoint

`PersistenceActor::get_profile` returns a complete core `ProfileAggregate` by
stable `ProfileId`, or `None` when the ID is absent. The actor reads the parent,
ordered tags, and ordered properties inside one worker-owned transaction, so a
caller never receives a mixed aggregate snapshot.

## Decode boundary

- Turso rows and values remain private to the persistence adapter.
- IDs and revisions retain their exact fixed-width big-endian encodings.
- Every stored label, limit, schema version, policy, preference, property, and
  secret reference is reconstructed through the authoritative bounded core
  constructors.
- Tag and property ordinals must be contiguous from zero. Core set constructors
  recheck cardinality, uniqueness, engine-neutral property shape, and schema.
- All six source kinds round-trip without resolution. Returned aggregate debug
  output remains redacted and contains no names, values, environment names,
  1Password identifiers, or breadcrumbs.
- Unknown discriminators, invalid cell types, malformed bounded values, schema
  drift, and invalid child ordering fail closed as `ProfileDecode`. Errors carry
  no database message or stored payload.

The read command uses the existing bounded actor queue, 30-second operation
deadline, and 35-second caller deadline. A failed decode rolls back its read
transaction before later actor commands proceed.

## Evidence

- Not-found lookup returns `None` without mutation.
- PostgreSQL, ClickHouse, and Redis fixtures round-trip exactly immediately
  after create and after clean database reopen.
- Fixtures exercise ordered tags, organization/preferences, TLS and safety,
  finite limits, every property source kind, and binary secret payloads.
- Debug rendering is checked against representative stored sensitive values.
- A database-valid but core-invalid whitespace-only profile name returns the
  metadata-only `ProfileDecode`; a following health command succeeds, proving
  the actor and transaction boundary remain usable.

## Deliberate boundary

Validated single-profile create/read is implemented. Revision-CAS replacement
is now implemented by
[`66-phase-2-saved-profile-replace.md`](66-phase-2-saved-profile-replace.md).
Bounded list/filter projections, deletion policy, source resolution,
dangerous-source user warnings, and write fault injection beyond transactional
child failure remain required. No UI consumes persisted profiles yet.

## Verification record

- `cargo test -p tablerock-persistence --test profile_create`: 3 passed.
- `cargo test --workspace --all-targets --locked`: 95 passed, 3 ignored.
- Workspace format, Clippy with warnings denied, rustdoc, diff, redaction, and
  provenance review: pass.

External concepts: transactional aggregate snapshot and strict adapter decoding
Public sources: no new external source; decoder derives from approved TableRock core contracts
Implementation source: TableRock-owned adapter, actor command, and real-file tests
Copied code/assets/text: none
