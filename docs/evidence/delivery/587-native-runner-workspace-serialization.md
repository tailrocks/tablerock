# Native runner workspace serialization

Date: 2026-07-21

## Failure

Native Nightly run 29838267439 overlapped Native Checkpoint run 29839995931 on
the same self-hosted macOS workspace. The workflows used different concurrency
groups, so both wrote the shared Cargo target tree concurrently. Nightly then
failed with Rust E0786 because `libhttp-*.rmeta` could not be memory-mapped as
valid metadata.

## Structural correction

`.github/workflows/native.yml` and
`.github/workflows/native-nightly.yml` now use the identical
`native-macos-${{ github.ref }}` concurrency group with
`cancel-in-progress: false`. GitHub Actions therefore serializes checkpoint and
nightly jobs for the same ref across workflow files. A queued proof is retained
rather than cancelling an in-flight evidence run.

This removes the architecture condition that allowed two jobs to mutate one
self-hosted checkout/target directory. It does not hide or retry corrupt build
output.

## Verification

- Failure log: Native Nightly 29838267439, step
  `Build canonical project and universal bridge`, Rust E0786 invalid metadata.
- The two workflow files have byte-identical concurrency group expressions.
- `actionlint` and YAML whitespace validation pass locally.
- Hosted serialized checkpoint/nightly proof remains pending after push.

## Provenance

No external product reference influenced this CI isolation fix. Evidence comes
from TableRock workflow definitions and hosted runner logs.
