# 576 — Native metadata fail-closed gate

Date: 2026-07-21

## Defect and structural fix

Hosted native checkpoint run `29834149579` passed its metadata step while the
metadata script printed `rg: command not found`. Bash does not exit for a
failed command used as an `if` condition, so the missing tool was interpreted
as "no forbidden linkage found."

The linkage rejection now uses macOS's baseline `grep -E`; the gate no longer
has an undeclared Homebrew dependency. The checkpoint path filter now includes
the metadata script, ensuring future changes to the gate run the gate.

## Verification

Run `29834149579` otherwise completed the canonical Xcode checkpoint, Release
archive, development app, and artifact upload. Its log exposed this defect and
is not accepted as metadata proof. The next hosted checkpoint must generate
non-empty metadata without a missing-command diagnostic before that proof is
claimed.

`bash -n scripts/record-native-artifact-metadata.sh` and
`git diff --check` pass locally.

## Provenance

Implementation source: TableRock-owned native delivery scripts and hosted logs.

TablePro influence: none; this is release-gate correctness.

Copied source, tests, identifiers, assets, strings, colors, geometry, layout
measurements, or key bindings: none.
