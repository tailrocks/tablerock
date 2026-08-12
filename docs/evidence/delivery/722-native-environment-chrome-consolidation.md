# Native environment-chrome consolidation

Date: 2026-08-13

Checkpoint: P4y — finish production presentation extraction

## Decision

The environment/safety halo now lives with context and status chrome in
`WorkbenchChrome.swift`. No production SwiftUI surface or AppKit adapter
remains in the legacy source region; `TableRockApp.swift` now retains only
development fixtures/scripted support plus their shared proof helpers pending
the dedicated Release-separation checkpoint.

## Bounds and failure truth

- Production, staging, development, and read-only wording, symbols, glass
  treatment, and non-color accessibility facts moved unchanged.
- Liquid Glass remains limited to functional chrome. Editor, data, inspector,
  and review content remain opaque.
- The Release fixture/scripted-backend debt is not claimed removed here. Its
  frozen source remains `TableRockApp.swift` until P6.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `scripts/verify-native-accessibility.sh` passed structural and real-window
  runtime gates.
- `scripts/verify-native-source-ownership.sh` passed.
- `git diff --check` passed.
- `TableRockApp.swift` decreased from 1,685 to 1,635 lines.

## Remaining work

Enforce the Presentation module and move its tests. Then remove every Release
fixture/scripted-backend symbol and path. Full visual/state/accessibility review
and repository readiness gates remain after those architecture checkpoints.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
