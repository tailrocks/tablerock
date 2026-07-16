# Phase 2 Saved Profile Replace Evidence

## Checkpoint

`PersistenceActor::replace_profile` accepts an expected revision and a
core-issued saved-profile token. Before I/O, the adapter requires the proposed
revision to be exactly the checked successor of the expected revision. Inside
one worker-owned transaction, the parent update compares both stable ID and the
exact 8-byte expected revision before replacing ordered children.

## Conflict contract

- A missing stable ID returns metadata-only `ProfileNotFound`.
- An existing ID with a different durable revision returns
  `ProfileStaleRevision`.
- A skipped, repeated, or exhausted proposed revision returns
  `ProfileInvalidRevision` before mutation.
- A successful compare-and-swap updates the complete aggregate and all ordered
  tags/properties atomically.
- Public errors expose no profile names, values, references, SQL, or database
  messages.

The persisted revision remains the authoritative concurrency fact. No
read-then-write gap exists outside the transaction, and no last-writer-wins
bypass is retained.

## Evidence

- A complete Redis aggregate advances revision 20 to 21 and reads back exactly.
- Reusing expected revision 20 is rejected as stale.
- Proposing revision 23 from expected revision 21 is rejected before I/O.
- Replacing a nonexistent PostgreSQL ID returns not-found.
- A trigger-injected ordered-tag failure after the parent update and child
  deletion reports `ProfileWrite`; subsequent read proves the complete prior
  ClickHouse aggregate and revision remain intact.

## Deliberate boundary

Create, strict single read, and revision-CAS replacement are implemented.
Deletion policy, bounded list/filter projections, source resolution, warnings,
and remaining crash/disk/permission fault matrices stay required.

## Verification record

- `cargo test -p tablerock-persistence --test profile_create`: 5 passed.
- `cargo test --workspace --all-targets --locked`: 97 passed, 3 ignored.
- Workspace format, Clippy with warnings denied, rustdoc, diff, redaction, and
  provenance review: pass.

External concepts: optimistic concurrency through transactional compare-and-swap
Public sources: no new external source; contract derives from approved TableRock revisions
Implementation source: TableRock-owned adapter, actor command, and real-file tests
Copied code/assets/text: none
