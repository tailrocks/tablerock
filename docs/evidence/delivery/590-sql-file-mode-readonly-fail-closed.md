# SQL-file mode-read-only fail-closed gate

Date: 2026-07-21

## Failure

CI run 29845160255 showed that `write_fails_closed_on_readonly_parent` could
write successfully after the parent directory was changed to mode `0555`.
Relying only on `File::create` lets a privileged process bypass mode bits, so
the product behavior and test varied with runner privilege.

## Correction

On Unix, the atomic SQL writer now reads the resolved parent metadata and
rejects the operation before creating its temporary file when all owner,
group, and other write bits are absent. Kernel authorization still handles ACL,
ownership, sandbox, and race-time failures. Non-Unix behavior remains the
existing OS authorization path.

This makes an explicitly mode-read-only directory fail closed even when the
process could technically override it, and preserves the invariant that a
rejected write leaves neither destination nor temporary file.

## Verification

- `cargo test -p tablerock-persistence
  sql_file::tests::write_fails_closed_on_readonly_parent`: passed.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy -p tablerock-persistence --all-targets -- -D warnings`:
  passed.
- `git diff --check`: passed.

## Provenance

No external product reference influenced this correction. Evidence comes from
TableRock's atomic-file invariant, Unix permission metadata, and hosted CI.
