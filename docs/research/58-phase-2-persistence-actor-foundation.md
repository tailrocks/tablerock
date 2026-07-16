# Phase 2 Persistence Actor Foundation Evidence

## Checkpoint

This checkpoint adopts exact `turso` 0.7.0 with default features disabled and
adds `tablerock-persistence`, the sole local storage boundary. One named worker
thread owns one current-thread Tokio runtime, `turso::Database`, and
`turso::Connection`. A bounded 32-command synchronous channel serializes all
access. No other crate receives a Turso type.

Context7 was attempted first and reported its monthly quota exhausted. API and
compatibility decisions were therefore checked against official Turso 0.7.0
docs.rs source, upstream repository/manual, and `COMPAT.md`.

## Local-only and feature proof

- Construction uses only `turso::Builder::new_local(path)`.
- `turso = { version = "=0.7.0", default-features = false }`; top-level `fts`,
  `mimalloc`, `sync`, test-helper, and experimental features are disabled.
- The crate graph contains no Hyper or Hyper-rustls activation. Turso 0.7.0
  currently carries its SDK/sync-kit crates unconditionally, but TableRock
  activates no network sync API and owns no remote URL, auth, or sync command.
- Encryption, MVCC, multi-process WAL, FTS, attach, vacuum, custom type/index,
  generated column, materialized view, and without-rowid builder switches are
  never called.
- `rusqlite` and `libsql` remain absent.

## Ownership, failure, and shutdown

- Database work has a 30-second worker timeout. Open, health, and explicit
  shutdown calls have a 35-second end-to-end caller bound; a full bounded queue
  fails immediately instead of blocking.
- Startup reports readiness only after local open, foreign-key enablement, and
  sequential migrations succeed.
- Public errors are a closed metadata-only code set. They never embed paths,
  SQL, database errors, profile data, or values.
- Explicit shutdown fully drains `PRAGMA wal_checkpoint(TRUNCATE)` and drops
  the connection/database on their owner thread before replying. `Drop` is
  nonblocking and only requests best-effort shutdown, so callers needing
  checkpoint evidence must call `shutdown`.
- Corrupt input fails closed and the hostile fixture proves its original bytes
  are not replaced by a new database.

## Sequential migrations

`crates/tablerock-persistence/MIGRATING.md` is the ordered index. Each migration
has one immutable zero-padded SQL file and a separate before/after explanation:

1. `0001-bootstrap` transactionally creates and seeds `schema_migrations`.
2. `0002-support-facts` transactionally creates the metadata-only support-fact
   table and records its migration in the same transaction.

Applied migrations are never rewritten. Every future incompatible schema
change adds the next SQL file, explanation file, and index row.

Startup validates that the ledger is an exact contiguous prefix of supported
migrations before mutation, then validates the exact current ledger after
migration. Future, malformed, and gapped ledgers fail closed.

## Compatibility evidence

Real temporary-file tests prove:

- first open, two migrations, exact schema version, foreign-key pragma, and
  integrity check;
- idempotent reopen;
- actual foreign-key rejection rather than flag inspection alone;
- ordinary transaction rollback;
- drained checkpoint, independent file copy, and restored-copy reopen;
- corrupt-file rejection without overwrite;
- future-version and gapped-ledger rejection before schema mutation.

## Supply chain

The selected Turso graph introduces BSD-3-Clause and ISC crates; both are
OSI-approved permissive licenses and are explicitly allowed. `cfg_block` 0.1.1
uses `license-file = "LICENSE"` rather than an SPDX manifest expression; the
packaged file is Apache-2.0 text with SHA-256
`408bbc4d10bdf74d8c9b74b64ea4910603257e24133da18edc4d16198e3b4010`.
`cargo deny` accepts the graph and reports that metadata-quality warning.

## Deliberate boundary

This is not the complete storage-proof exit. Profile encoding/CRUD, saved-only
token enforcement at actor commands, transactional revision CAS, retention,
disk-full/interrupted-migration injection, crash subprocess matrix, independent
backup manifest, package-size/release artifact, and clean-machine macOS/terminal
deployment remain required before profile persistence is claimed.

Subsequent checkpoint `60-phase-2-persistence-ownership-recovery.md` closes the
normalized process-local ownership and interrupted transactional migration
items while retaining the other listed gates.

## Verification record

- `cargo test -p tablerock-persistence`: 4 passed.
- `cargo clippy -p tablerock-persistence --all-targets --locked -- -D warnings`:
  pass.
- `cargo test --workspace --locked`: 88 passed, 3 ignored.
- Workspace format, Clippy, rustdoc, `cargo deny`, `gitleaks`, English-script,
  and complete-diff gates: pass. Turso introduces expected duplicate-version
  warnings plus the documented `cfg_block` license-metadata warning.

External concepts: serialized actor, transactional migrations, local embedded database
Public sources: <https://docs.rs/turso/0.7.0/turso/struct.Builder.html>,
<https://github.com/tursodatabase/turso/blob/main/docs/manual.md>, and
<https://github.com/tursodatabase/turso/blob/main/COMPAT.md>
TableRock requirements: research 10, 20, 30, 31, and 32
Implementation source: TableRock-owned actor, migrations, and real-file tests
Copied code/assets/text: none
