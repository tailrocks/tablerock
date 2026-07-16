# Phase 2 Profile Aggregate Evidence

## Checkpoint

This Phase 2 tracer composes `ProfileConnectionSnapshot`, durability,
organization, and connection-behavior preferences into schema-version-1
`ProfileAggregate`. It is the baseline durable profile shape for the upcoming
Turso migration checkpoint; later transport or advanced settings require
explicit sequential schema migrations rather than unversioned fields.

## Durability and organization

- Durability is closed. Only `Saved` can produce the non-constructible
  `PersistableProfile<'_>` token accepted by the future actor; `Temporary`
  produces no token.
- Temporary aggregates reject group, tags, favorite, or nonzero saved order.
  The persistence actor must accept only `PersistableProfile`; presentation
  cannot promote a temporary connection by writing it directly.
- Optional group names are nonempty, control-free, and at most 128 UTF-8 bytes.
- Tags are nonempty, control-free, case-sensitive, exactly unique, at most 64
  UTF-8 bytes each, and limited to 32 per profile.
- Favorite and explicit unsigned order are stable organization facts. Group and
  tag text is omitted from `Debug`; only presence/count/order facts remain.

## Preferences

The fixed baseline profile preferences are manual or bounded-automatic
reconnect selection, intent-only last-context restoration, and preferred page
rows in `1..=500`. Reconnect remains a preference, not authority to retry:
engine policy must stop on authentication failure and never replay an ambiguous
write. Page preference never weakens the independent page/command/process
budgets.

## Revision replacement

`validate_replacement` is a non-consuming compare-and-swap gate. It requires:

- caller expected revision equals current revision;
- stable profile identity is unchanged;
- durability is unchanged by ordinary replacement;
- proposed revision is exactly the checked monotonic successor.

Stale, cross-identity, durability-changing, skipped, and exhausted revisions
fail with typed metadata-only errors. Validation borrows both aggregates, so
rejection does not drop or duplicate their non-cloneable secret ownership. The
persistence actor must repeat this check transactionally against durable state.

## Deliberate boundary

No file, Turso database, migration SQL, CRUD actor, resolver, driver, reconnect
loop, context state, logging, telemetry, FFI encoding, or UI projection exists
here. Saved/temporary conversion is an explicit future operation, not ordinary
replacement. SSH and evidence-backed advanced settings remain later versioned
aggregate extensions.

## Evidence

- Saved fixtures produce a persistable token; temporary fixtures cannot.
- Hostile organization tests reject empty/control/oversized labels, duplicate
  tags, and more than 32 tags.
- Preference tests prove exact page-size minima/maxima and reject zero/overflow.
- Replacement tests prove success plus stale, cross-identity, durability,
  skipped-revision, and exhausted-revision rejection.
- Unknown aggregate schema versions fail closed.
- Debug fixtures prove profile name, endpoint, group, and tags absent.

## Verification record

- `cargo test -p tablerock-core --test profile_aggregate`: 5 passed.
- `cargo clippy -p tablerock-core --all-targets --locked -- -D warnings`: pass.
- `cargo test --workspace --locked`: 84 passed, 3 ignored.
- Workspace format, Clippy, rustdoc, `cargo deny`, `gitleaks`, English-script,
  and complete-diff gates: pass. The already-allowed `hashbrown` duplicate is
  unchanged.

External concepts: aggregate root, unforgeable persistence eligibility, and optimistic CAS
Public sources: none required; policy derives from approved TableRock research
TableRock requirements: research 02, 06, 10, 14, 30, 31, and 32
Implementation source: TableRock core architecture and independent tests
Copied code/assets/text: none
