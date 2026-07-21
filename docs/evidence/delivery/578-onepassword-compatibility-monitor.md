# 578 — 1Password CLI compatibility monitor

Date: 2026-07-21

## Decision

The scheduled native gate installs current stable 1Password CLI 2.35.0 and
verifies the exact command surface TableRock invokes: `op read`,
`--no-newline`, and the global `--account` selector. Pinning the asserted
version makes a newer stable cask fail visibly until reviewed and adopted.

This is syntax/tool availability proof only. CI has no operator account or
secret and therefore makes no authenticated-secret claim. Unit and integration
ports continue to prove bounded output, timeout, exit failure, missing CLI,
empty output, and redaction without exposing a real credential.

## Existing recurring coverage

The daily CI schedule runs `cargo outdated --workspace --root-deps-only`, which
checks registry dependencies and the exact TermRock Git source, plus advisory,
license/source, and GitHub Action pin audits. The daily native schedule records
Rust, Swift, Xcode, and macOS versions; exercises the supported server and
native matrices; and builds the canonical universal archive with XcodeGen.

## Verification

Local current-cask inspection and command probes report 2.35.0 and expose both
required flags. `actionlint .github/workflows/native-nightly.yml` and
`git diff --check` pass. Hosted scheduled proof remains required after push.

## Provenance

Implementation sources: TableRock's `OpCliReader`, Homebrew cask metadata, and
official 1Password CLI command documentation.

TablePro influence: none; this is credential-tool compatibility monitoring.

Copied source, tests, identifiers, assets, strings, colors, geometry, layout
measurements, or key bindings: none.
