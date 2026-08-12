# Native source ownership gate

Date: 2026-08-12

Checkpoint: P1 — make native gates ownership-aware

## Decision

Native structural verifiers now query shared presentation or whole-production
source scopes instead of one filename. Presentation scope currently includes
`TableRockApp` and will automatically include `TableRockPresentation` when that
module is introduced. Whole-production scope also covers Feature and Bridge for
explicit integration assertions. Regex, fixed-string, and total-match-count
operations have one tested helper.

A separate ownership gate freezes the 12,231-line monolith ceiling, rejects
production imports of Design Lab, rejects forbidden Feature/Bridge/Presentation
dependency directions, proves SwiftPM and XcodeGen Design Lab isolation,
rejects direct verifier coupling to `TableRockApp.swift`, and prevents existing
Release fixture debt from gaining symbols or spreading to new source files.

The 33 existing fixture environment symbols and their two current source files
are explicit temporary debt. Removing debt is permitted; adding debt is not.
Checkpoint P6 will make both debt sets empty and prove no fixture symbols remain
in the Release executable.

## Root cause and repaired drift

The old scripts encoded implementation ownership as one filename. They were
also absent from the native workflow path filter, so a verifier assertion could
become stale when production presentation changed without scheduling the native
checkpoint workflow. This reproduced the monolith's enabling condition inside
its tests: behavior and verification structure could drift independently.

Running every affected gate exposed stale assertions for:

- typed result-cell accessibility values;
- SwiftUI workbench versus AppKit window identifiers;
- workspace-tab bridge-record conversion;
- history retention client naming;
- current group label/menu formatting and reconnect bridge call;
- last-activated result-column selection and the redesigned typed value
  inspector;
- query-only result-copy command enablement.

Each assertion now names the current, equal-or-stronger production contract.
No required assertion or runtime proof was removed. The native workflow path
filter now includes all `scripts/**`, so future gate/helper changes schedule the
checkpoint workflow.

## Bounds and failure truth

- The helper searches production Swift source only. It does not search Design
  Lab or tests, which could create false positives.
- Existing Release fixture behavior is quarantined, not approved. The debt
  manifests are ceilings and cannot justify new fixture routes.
- The monolith is not yet split. This checkpoint makes extraction observable
  and prevents the enabling condition from growing.
- Database, bridge, safety, and presentation runtime behavior did not change.
- CI execution remains pending until the pushed commit is processed by GitHub.

## Evidence

The following passed locally on macOS 26.5.2, Xcode 26.6, Swift 6.3.3, Docker
29.4.0:

- `bash -n scripts/lib/native-source-verifier.sh scripts/test-native-source-verifier.sh scripts/verify-native-*.sh`
- `scripts/test-native-source-verifier.sh`
- `scripts/verify-native-source-ownership.sh`
- `scripts/build-native-app.sh`
- `scripts/verify-native-accessibility.sh`
- `scripts/verify-native-clickhouse-structure.sh`
- `scripts/verify-native-csv-import.sh`
- `scripts/verify-native-history.sh`
- `scripts/verify-native-maintenance.sh`
- `scripts/verify-native-multi-window.sh`
- `scripts/verify-native-object-tabs.sh`
- `scripts/verify-native-profile-editor.sh`
- `scripts/verify-native-profile-groups.sh`
- `scripts/verify-native-query-tabs.sh`
- `scripts/verify-native-redis-key-view.sh`
- `scripts/verify-native-redis-overview.sh`
- `scripts/verify-native-result-copy.sh`
- `scripts/verify-native-saved-queries.sh`
- `scripts/verify-native-sql-files.sh`
- `scripts/verify-native-structure.sh`
- `scripts/verify-native-value-inspector.sh`
- YAML parsing for the three changed native workflows and XcodeGen project spec
- `git diff --check`

PostgreSQL, ClickHouse, and Redis runtime fixtures passed. Accessibility,
multi-window, file, tab, history, profile, copy/export, inspector, maintenance,
structure, and import behavior passed. All helper modes also passed a synthetic
two-file extraction test.

## Remaining work

P2 extracts application integration and bridge conversion ownership. Each
subsequent extraction must keep this gate and affected runtime verifiers green.
P6 removes the frozen Release fixture debt entirely.

## Clean-room provenance

This checkpoint changes architecture and verification only. TablePro public
user-visible organization influenced the confirmed workbench direction; no
TablePro source, tests, comments, bundle internals, branding, assets, copy, or
proprietary fixtures were used.
