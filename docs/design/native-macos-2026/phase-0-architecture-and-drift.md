# Native macOS 2026 phase 0: architecture and drift

Status: complete audit; no production-interface changes authorized  
Date: 2026-08-12

This audit defines the safe boundary for a separate `TableRockDesignLab` and
records the first operator gate. It does not approve a concept or authorize
production migration.

## Operator decisions

- TablePro's observable, publicly presented macOS interface is the primary
  interaction and composition reference. TableRock may adapt its native
  density, workbench hierarchy, toolbar/sidebar/grid rhythm, and familiar
  macOS conventions.
- TableRock keeps its own identity, copy, safety behavior, fixtures, and code.
  No TablePro source, tests, comments, bundle internals, branding, proprietary
  assets, stored credentials, or real database content may be inspected or
  copied.
- `design/TableRock-Native-Concepts.sketch` is operator-rejected and must have
  zero influence on this design exploration. The operator explicitly directed
  its complete removal from repository history on 2026-08-12.
- Production UI remains unchanged until the operator selects a captured
  concept and separately confirms its refinement.

## Verified baseline

| Area | Observed baseline |
|---|---|
| Host | macOS 26.5.2 (25F84), Apple silicon |
| Xcode and SDK | Xcode 26.6 (17F113), macOS SDK 26.5 |
| Swift | 6.3.3; package tools version 6.2; macOS 26 minimum |
| Project generator | XcodeGen 2.46.0, current Homebrew stable on audit date |
| Package verification | `swift test`: 56 XCTest cases run, 4 live-server cases skipped, 0 failures; 14 Swift Testing tests passed |
| Native targets | `TableRockFeature`, `TableRockBridge`, `TableRock`, and their tests; no design-lab target |

## Responsibility map and enabling condition

`native/Sources/TableRockApp/TableRockApp.swift` is a 12,231-line compilation
unit containing approximately 95 type declarations. Its responsibilities are
not separated at module boundaries:

| Approximate lines | Responsibility |
|---:|---|
| 36–314 | Editor drafts, appearance fixtures, commands, and actions |
| 315–1,802 | Scripted and live workbench backends |
| 1,803–1,984 | Platform ports and application model |
| 1,985–2,527 | App/window composition and fixture surfaces |
| 2,528–6,661 | Presentation types and `BridgeModel` workflow state |
| 6,662–10,710 | Connections, workbench, query, object, review, sheet, grid, editor, and profile presentation |
| 10,711–12,231 | AppKit outline/grid/editor adapters and find/replace engine |

The architectural condition permitting broad UI regressions is this single
target/file ownership: presentation, backend selection, bridge state, platform
I/O, fixture behavior, AppKit controls, and app lifecycle can change together
without an enforced dependency boundary. App tests use `@testable import
TableRock`, further coupling verification to the application target.

Eventually migrating an approved design therefore requires a behavior-
preserving extraction of native presentation contracts and views from live
backend/platform adapters. That refactor is deliberately deferred. Performing
it before concept approval would change production structure before the gate.

## Drift report

| Expected capability | Repository state | Required pre-gate action |
|---|---|---|
| Isolated design exploration | No `TableRockDesignLab` product or target | Add an executable/app target with no production dependencies |
| Immutable preview data | Production fixture and live paths share the app source | Define lab-owned immutable values only |
| Five structural concepts | No current concept runtime | Build five distinct compositions, not color variants |
| Repeatable comparison | No concept/surface/appearance launch contract | Add deterministic launch arguments and captures |
| Accessibility evidence | Production supports system appearance but has no concept matrix | Exercise light/dark, Reduce Transparency, Increase Contrast, and Reduce Motion in the lab |
| Liquid Glass discipline | Existing production UI predates this gate | Use glass only for navigation and top-level controls in the lab; keep data/editor content opaque |
| Design-reference truth | The rejected Sketch conflicted with operator intent | Remove it from history and exclude it from all requirements and implementation decisions |
| Reference-policy documentation | Historical evidence predates the broadened clean-room policy | Record new public-reference provenance without rewriting historical claims |

## Design-lab boundary

`TableRockDesignLab` must own only:

- immutable, invented connection, schema, query, result, and review fixtures;
- concept and surface selection values;
- reusable lab-local views and AppKit/SwiftUI presentation helpers;
- deterministic appearance/accessibility preview settings;
- capture and isolation tests.

It must not depend on or name `TableRockBridge`, Rust, UniFFI, persistence,
networking, production models, `BridgeModel`, `WorkbenchBackend`, credentials,
or a database client. Local transient selection state is permitted; external
I/O is not.

The production application source is outside the pre-gate edit boundary.

## Pre-gate acceptance contract

The first operator gate opens only after all of the following exist and pass:

1. Five high-fidelity, structurally distinct native concepts.
2. Each concept covers Connections, Connection Setup, Data Grid, SQL + Results,
   and Change Review using the same immutable fixture vocabulary.
3. Deterministic light and dark captures, plus accessibility captures or
   runtime evidence for Reduce Transparency, Increase Contrast, and Reduce
   Motion.
4. A verified target-dependency and forbidden-symbol isolation check.
5. A build/test result on the pinned native toolchain.
6. Reference provenance describing which public TablePro patterns and Apple
   platform guidance influenced each concept.

At that point work stops. Only the operator can select a concept. Production
migration, production-source refactoring, real data wiring, persistence, and
network behavior are explicit non-goals before selection and refinement
confirmation.

## Primary platform guidance

The audit used Apple's current macOS design and API material: [Materials and
Liquid Glass](https://developer.apple.com/design/human-interface-guidelines/materials),
[Designing for macOS](https://developer.apple.com/design/human-interface-guidelines/designing-for-macos/),
[Applying Liquid Glass to custom
views](https://developer.apple.com/documentation/SwiftUI/Applying-Liquid-Glass-to-custom-views),
and [AppKit in the new design
system](https://developer.apple.com/videos/play/wwdc2025/310/). The installed
macOS 26 SDK headers were the authority for exact API availability.

Glass is a functional navigation/control layer, not a data-content treatment.
The lab will prefer system sidebars, toolbars, search, sheets, controls, and
semantic colors; use regular glass sparingly; avoid clear glass over ordinary
workbench content; and respect system accessibility settings.

Public TablePro references were limited to its [product
page](https://tablepro.app/) and public documentation for the [SQL
editor](https://docs.tablepro.app/features/sql-editor) and [data
grid](https://docs.tablepro.app/features/data-grid).
