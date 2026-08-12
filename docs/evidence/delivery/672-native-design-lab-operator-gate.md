# Native Design Lab first operator gate

Date: 2026-08-12  
Gate: open; operator decision required

## Outcome

`TableRockDesignLab` now presents five structurally distinct, navigable native
macOS concepts using the same immutable PostgreSQL, ClickHouse, and Redis
fixture vocabulary. It is a separate Swift executable and Xcode app with no
production, bridge, Rust, persistence, network, credential, or database-client
dependency.

The [operator review and capture index](../design-lab/README.md) contains the
runnable command, representative captures, interaction summary, component and
material maps, weighted review, strengths, weaknesses, recommendation,
accessibility evidence, limits, and required decision.

## Proof

```text
./scripts/verify-native-design-lab.sh
  Swift executable build: pass
  TableRockDesignLabTests: 5 pass
  Xcode dependency graph: TableRockDesignLab (no dependencies)
  Xcode app build: pass
  TableRockDesignLabUITests: 3 pass

swift test --package-path native
  XCTest: 56 pass, 4 configured live-server skips
  Swift Testing: 19 pass

capture matrix: 55 running-window images
checksums: 55 verified
window sizes: 1280×760 / 1440×900 / 1720×1040
```

The UI tests cover semantic accessibility audit, native table selection,
context menu, inspector updates, connection and destructive-review sheets,
typed `APPLY`, menu commands, engine switching, deterministic fixtures, and
exact minimum sizing.

## Boundary proof

- Design Lab package target dependencies: zero.
- Design Lab Xcode application dependencies: zero.
- Production `native/Sources/TableRockApp/TableRockApp.swift`: unchanged.
- Static invented fixtures only; no real SQL, values, hosts, users, or secrets.
- Rejected `design/TableRock-Native-Concepts.sketch`: absent from branch and
  reachable repository history; no design influence.
- Phase 10 production/refinement work: not started.

## Provenance

TablePro public screenshots and documentation informed broad, user-visible
database-workbench organization only. Apple components and the installed
stable SDK supplied implementation authority. No external source, tests,
comments, internals, branding, assets, product copy, or proprietary fixtures
were inspected or copied.

## Stop condition

The implementation recommendation is Native Workbench, but the agent has not
selected it. Only the operator may select a concept, request changes, or define
a remix. Production UI remains frozen until that decision and the subsequent
refined-concept confirmation.
