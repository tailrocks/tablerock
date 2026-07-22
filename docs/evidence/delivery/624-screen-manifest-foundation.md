# Canonical screen-manifest foundation

Date: 2026-07-22

## Delivered

- `screen-manifest.tsv` inventories 38 canonical TableRock surfaces with
  stable IDs, kind, clients, engines, entry/exit paths, actions, focus order,
  state profile, product/plan ownership, implementations, tests, evidence,
  status, and explicit remaining gap.
- `screen-state-profiles.tsv` expands every row onto the required normal,
  empty, loading, partial, stale, disabled, unsupported, validation,
  permission, disconnected, reconnecting, error, destructive-confirmation,
  narrow, large-data, and recovery vocabulary.
- `screen_manifest` runs in the ordinary `tablerock-core` test suite. It
  rejects malformed rows, duplicate IDs, unknown profiles/statuses, stale
  paths, missing client implementation/test links, uncovered product screen
  documents, dishonest `proven` rows, and accidental inventory shrinkage.

Every initial row is deliberately `partial`. Broad existing test/evidence
links establish ownership, not direct per-state proof. Each row names its next
replay or implementation gap. Completion still requires narrowing links to
direct cases, exercising all applicable states, inspecting visual/runtime
artifacts, and changing status only with evidence.

## Verification

```text
mise exec -- rtk cargo test -p tablerock-core --test screen_manifest
# 1 passed

mise exec -- rtk cargo fmt --all --check
git diff --check
# clean
```

## Provenance

No external product source, test, identifier, text, asset, screenshot,
geometry, color, or key binding influenced this manifest. Inventory derives
from TableRock product documents, plans, implementation, tests, and evidence.
The required final TablePro public-workflow clean-room review remains open.
