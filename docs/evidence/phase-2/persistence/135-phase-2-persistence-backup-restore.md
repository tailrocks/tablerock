# Phase 2 Persistence Backup And Restore Evidence

Date: 2026-07-17

## Decision

TableRock now owns one offline backup and restore path inside
`tablerock-persistence`. It does not expose raw file copying as a supported
workflow and does not add a second SQLite implementation. Backup and restore
use the existing normalized process-local `PathLease`, Turso 0.7 local open,
schema migration validation, integrity check, and WAL checkpoint behavior.

`create_backup` requires an existing regular source file, absent backup and
manifest destinations, and no live actor for either database path. While both
leases remain held it opens the source through the authoritative Turso path,
applies/validates the exact migration prefix, checkpoints the WAL, requires
healthy integrity, and closes Turso before copying. It streams the database in
64 KiB chunks with a 512 MiB hard limit into a create-new temporary file,
flushes and synchronizes it, then renames it within the destination directory.
The copied database is independently reopened and health-checked before its
manifest is published.

The version-1 sidecar manifest contains only format version, schema version,
exact file length, and SHA-256. It contains no database path, profile metadata,
secret reference, query, value, or timestamp. The manifest is bounded to 512
bytes, strictly ordered, rejects unknown versions/trailing fields, and is
written through its own synchronized create-new temporary file. SHA-256 uses
latest stable `sha2` 0.11.0 with default features disabled; Cargo registry and
official RustCrypto metadata report Rust 1.85 minimum and MIT OR Apache-2.0.
Context7 was attempted first and reported its monthly quota exhausted.
Database and manifest files are created with owner-only `0600` permissions on
the supported Unix platforms before any byte is written.

`restore_backup` first verifies manifest version, size, and complete SHA-256,
then reopens the backup through Turso and requires matching schema/integrity.
It copies only to an absent destination, verifies the copy digest, reopens the
restored file independently, and requires identical health. It never removes,
renames, or overwrites an existing database. Replacing a failed original is a
later explicit operator workflow; the Rust recovery primitive deliberately
cannot perform that destructive step.

## Failure, cancellation, and safety

- The workflow is synchronous and bounded; it has no presentation cancellation
  surface yet. Once called, it finishes or returns one metadata-only
  `PersistenceError` without exposing paths or storage contents.
- A live source or destination actor returns `DatabaseBusy`; aliases cannot
  bypass normalized ownership.
- Existing backup, manifest, and restore destinations fail closed. No
  last-writer-wins or overwrite option exists.
- Source changes cannot race the copy inside this process because the source
  lease spans checkpoint, close, copy, verification, and manifest publication.
- Size changes during copy and every digest/health mismatch fail verification.
- Turso open/checkpoint/integrity verification retains the actor's 30-second
  operation timeout; filesystem copy is byte-bounded rather than cancellable.
- Temporary artifacts created by a failed unpublished operation are removed.
  Published or pre-existing files are never automatically deleted.
- A directory-sync failure after publication reports failure and retains the
  artifacts for inspection; callers must not infer absence from an error.
- Backup creation and restoration contain no retry loop. An uncertain
  filesystem outcome is never automatically repeated.

## Evidence

Real-file integration tests prove checkpointed backup, strict manifest round
trip, independent restore, exact file equality, restored Turso health,
live-source rejection, no-overwrite behavior, failed-original preservation,
tamper detection before target creation, and malformed-manifest rejection.

This closes Phase 2's independent backup-manifest and verified offline
backup/restore primitive. Disk-full/permission fault injection, crash during
backup/manifest publication, cross-process coordination, package-size and
clean-machine evidence, and the later operator replacement UX remain open.

## Provenance

External concepts: offline database backup, digest manifest, atomic publish
Public sources: <https://docs.rs/turso/0.7.0/turso/struct.Builder.html>,
<https://docs.rs/sha2/0.11.0/sha2/>, and
<https://github.com/RustCrypto/hashes/tree/master/sha2>
TableRock requirements: research 10, 20, 30, 31, and 32
Implementation source: TableRock-owned lease, streaming copy, manifest parser,
and real-file tests
Copied code/assets/text: none
