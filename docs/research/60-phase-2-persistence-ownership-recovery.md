# Phase 2 Persistence Ownership And Recovery Evidence

## Checkpoint

The persistence boundary now permits exactly one live `PersistenceActor` for a
normalized database path inside a TableRock process. Existing paths are
canonicalized. New paths canonicalize their existing parent before the filename
is joined, so relative and `.` aliases cannot create two owners.

The lease is acquired before the worker starts and is held by the worker, not
the presentation caller. Startup failure, channel closure, worker exit, and
explicit shutdown release it. Explicit shutdown releases ownership before its
success reply, making immediate reopen deterministic. Registry poisoning fails
closed; no caller can clear or forge a lease.

This is process-local serialization, not a daemon, RPC service, or claim that
two independently launched TableRock processes may safely share one file.
Application-level single-instance/file coordination remains a distribution
checkpoint.

## Recovery rules

1. Never delete, truncate, rename, or recreate a database automatically after
   open, migration, decode, integrity, or corruption failure.
2. An existing migration ledger must be the exact contiguous prefix supported
   by the binary. Future, empty, malformed, and gapped ledgers fail closed.
   Open the file with a binary supporting its newest migration; never downgrade
   or edit the ledger manually.
3. Each migration and its ledger insert share one database transaction. If a
   process stops after schema statements but before commit, restart observes
   the previous ledger/schema and reapplies the complete migration.
4. `DatabaseBusy` means an actor still owns the normalized path. Shut that actor
   down or wait for its worker to exit. Never bypass ownership with a path
   alias.
5. A verified backup copy is made only after explicit `shutdown` completes its
   WAL truncate checkpoint. Restore only while no actor owns either path, keep
   the failed original, and validate the restored copy by opening it and reading
   health before replacing user state.
6. `Drop` requests nonblocking best-effort closure; it is not backup evidence.
   Workflows requiring a durable checkpoint must use explicit `shutdown`.

Automatic repair remains forbidden because a repair attempt can destroy the
only forensic copy. A later user-facing recovery workflow must copy first,
identify the failed invariant with metadata-only diagnostics, and require an
explicit operator decision before replacement.

## Evidence

- Opening the same existing file through its canonical spelling and a `.` alias
  rejects the second actor.
- After explicit shutdown, the same path reopens immediately and remains
  healthy.
- A real Turso transaction creates the second migration's table and is dropped
  before commit. Reopen rolls it back, applies migration `0002` completely, and
  reports exact schema version 2.
- Existing future/gap and corrupt-file fixtures continue to fail closed.

## Remaining storage proof

Disk-full, permission/read-only injection, crash subprocesses at additional
boundaries, integrity recovery UX, retention/private history, independent
backup manifests, Release package size, clean-machine deployment, and
cross-process application ownership remain open. Profile persistence remains
unclaimed until its encoding, saved-token gate, and transactional revision CAS
are implemented after the applicable storage prerequisites.

## Verification record

- `cargo test -p tablerock-persistence`: 6 passed.
- Targeted Clippy: pass.
- `cargo test --workspace --all-targets --locked`: 90 passed, 3 ignored.
- Workspace format, Clippy, rustdoc, dependency policy, leak scan, and diff
  checks: pass.

External concepts: normalized resource ownership, transactional migration recovery
Public sources: no new external source; behavior derives from approved research 10, 30, 31, and 32
Implementation source: TableRock-owned actor and real-file tests
Copied code/assets/text: none
