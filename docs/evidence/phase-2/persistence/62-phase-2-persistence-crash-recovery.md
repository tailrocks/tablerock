# Phase 2 Persistence Crash Recovery Evidence

## Checkpoint

A dedicated integration-test subprocess opens a fresh real Turso file, applies
all supported migrations, verifies foreign keys and integrity, then terminates
with `abort`. This deliberately bypasses `PersistenceActor::Drop`, explicit
shutdown, WAL checkpoint, normal Rust unwinding, and worker-thread cleanup.

The parent process requires the child to fail, then opens the same file through
a new actor. Reopen must report exact schema version 3, enabled foreign keys,
and `PRAGMA integrity_check = ok` before an explicit checkpointed shutdown.

## Safety and bounds

- The fixture path is unique to the parent test process and passed through a
  test-only environment variable. Product code has no crash switch.
- Child output is captured and never incorporated into a public error.
- Cleanup covers the database plus possible `-wal` and `-shm` companions.
- Recovery never deletes or mutates a failed original automatically. This test
  proves compatible reopen after a known committed state; it does not authorize
  repair after arbitrary corruption.

## What this proves

- All three committed startup migrations survive abrupt whole-process death.
- Reopen does not depend on actor destructor execution or a preceding manual
  checkpoint.
- The new process rebuilds ownership state rather than inheriting stale
  process-local leases.
- Schema, foreign-key, and integrity health remain readable after recovery.

## Remaining crash matrix

Fault injection must still cover process death during each migration boundary,
durable profile/history writes, backup creation, and restoration-intent updates.
Disk-full, permission/read-only, damaged-page recovery UX, and crash injection
during backup publication remain open. Checkpoint
`135-phase-2-persistence-backup-restore.md` closes the independent manifest and
verified offline restore primitive, but it cannot infer crash-boundary evidence
from the successful path.

## Verification record

- `cargo test -p tablerock-persistence --test crash_recovery`: 2 passed.
- `cargo test -p tablerock-persistence`: 8 passed across 5 suites.
- Targeted Clippy: pass.
- `cargo test --workspace --all-targets --locked`: 92 passed, 3 ignored.
- Workspace format, Clippy, rustdoc, dependency policy, leak scan, and diff
  checks: pass.

External concepts: abrupt process termination and transactional crash recovery
Public sources: no new external source; evidence exercises the approved local Turso architecture
Implementation source: TableRock-owned subprocess fixture
Copied code/assets/text: none
