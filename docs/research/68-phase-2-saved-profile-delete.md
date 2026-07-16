# Phase 2 Saved Profile Delete Evidence

## Checkpoint

`PersistenceActor::delete_profile` requires both stable `ProfileId` and the
caller's expected `Revision`. One worker-owned transaction deletes only when
both exact fixed-width values match the durable row.

## Ownership and conflict contract

- A mismatched revision returns metadata-only `ProfileStaleRevision` and leaves
  the complete aggregate unchanged.
- A missing or already-deleted ID returns `ProfileNotFound`.
- Success removes the profile parent and its profile-owned ordered tag/property
  rows through declared foreign-key cascades.
- No unconditional delete API or last-writer-wins fallback exists.
- History, saved queries, active work, and other future entities must not become
  cascade-owned by a profile. Their retention/detachment policy remains a
  separate schema decision before those tables exist.

The command retains the bounded actor queue and operation/caller deadlines.
Errors contain no name, value, secret reference, SQL, or database message.

## Evidence

- A stale deletion attempt leaves a complete PostgreSQL aggregate readable.
- A nonexistent ID fails without mutation.
- An exact revision deletion removes the parent, two tags, and nine property
  rows; independent Turso inspection finds all three profile tables empty.
- Repeated deletion returns not-found.
- Clean reopen confirms the profile remains absent.

## Deliberate boundary

Saved-profile create, strict single read, revision-CAS replacement, and
revision-CAS deletion are implemented. The bounded base list is now implemented
by [`69-phase-2-bounded-profile-list.md`](69-phase-2-bounded-profile-list.md).
Filtered projections, source resolution, user warnings, unrelated-entity
retention, and remaining fault matrices stay required.

## Verification record

- `cargo test -p tablerock-persistence --test profile_create`: 6 passed.
- `cargo test --workspace --all-targets --locked`: 98 passed, 3 ignored.
- Workspace format, Clippy with warnings denied, rustdoc, diff, English-only,
  redaction, and provenance review: pass.

External concepts: optimistic-concurrency deletion and aggregate ownership
Public sources: no new external source; contract derives from approved TableRock revisions and ledger
Implementation source: TableRock-owned adapter, actor command, and real-file tests
Copied code/assets/text: none
