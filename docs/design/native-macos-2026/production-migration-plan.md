# Native Workbench production migration plan

Status: approved design; implementation package ready

Date: 2026-08-12

Decision owner: operator

Implementation scope: native macOS production application

This is the zero-context implementation package for replacing the current
native presentation with the operator-confirmed Native Workbench. It is also
the structural repair for the condition that allowed one 12,231-line source
file to own application lifecycle, platform I/O, bridge calls, mutable
presentation state, fixtures, SwiftUI views, and AppKit adapters.

The approved runtime and its evidence are fixed at the revisions recorded in
[`decision-native-workbench.md`](decision-native-workbench.md). The Design Lab
is reference evidence only. Production must not import it, compile its source,
read its fixtures, or share its mutable state.

## Authority and resolved questions

| Question | Resolution |
|---|---|
| Is production migration authorized? | Yes. The operator explicitly stated: “This native concept is the production design TableRock should implement.” |
| What is the design authority? | Apple platform behavior plus the confirmed TableRock Native Workbench runtime, contracts, and captures. TablePro public material supplied broad clean-room direction only. |
| Is any Sketch file authoritative? | No. No Sketch frame is approved. The operator-rejected Sketch was purged and must not be restored or used. |
| May production depend on Design Lab? | No. Approved structures are reimplemented in production-owned files. Design Lab remains dependency-free evidence. |
| Where do database semantics and safety live? | Rust core, engine, persistence, and FFI layers. Swift presents typed outcomes and intents; it does not reimplement database or safety rules. |
| What is the bridge shape? | Embedded synchronous UniFFI only. No daemon, RPC, manual C ABI, WebView, or alternate bridge. Blocking calls are initiated outside SwiftUI rendering and coordinated by a `@MainActor` store. |
| What owns UI state? | One production presentation store owns mutable navigation, toolbar, inspector, sheet, alert, tab, and selection state. Views derive projections and send intents. |
| What is the SwiftUI/AppKit boundary? | SwiftUI owns application composition. `NSOutlineView`, `NSTableView`, and `NSTextView` remain narrow adapters where native database-tree, dense-grid, or editor behavior requires them. |
| Where may Liquid Glass appear? | Functional system chrome only: system toolbar, sidebar/navigation chrome, approved controls, sheets, and menus. Data, editor, result, inspector content, and review content stay opaque. |
| What happens to scripted fixture behavior? | It moves behind development/test compilation and deterministic visual-QA routes. Release output must not contain scripted backend selection or fixture environment routes. |
| Must old presentation coexist indefinitely? | No. It may coexist only inside an active replacement checkpoint. Superseded code is removed after replacement coverage passes. |
| How are structural rollbacks performed? | Forward-only recovery: revert a checkpoint with a new signed commit or repair forward. Do not rewrite published implementation history. |

### Explicitly deferred external gates

- Developer ID signing, notarization, stapling, and clean-machine installation
  remain operator-credential gates already recorded by roadmap evidence. Local
  architecture, application, accessibility, and visual work continues without
  claiming that distribution proof.
- Hosted VoiceOver and IME evidence remains part of final native parity. A
  missing Accessibility permission or non-interactive session is a blocker,
  not a pass.

No other design question is open. If migration reveals an Apple API limitation,
an unapproved custom component, or a required domain-ownership change, stop and
record bounded research before changing the design or Rust/Swift boundary.

## Baseline and drift

The pre-refactor baseline on 2026-08-12 is:

- `native/Sources/TableRockApp/TableRockApp.swift`: 12,231 lines and 108
  top-level declarations;
- `BridgeModel`: approximately 3,943 lines of workflow and presentation state;
- `native/Sources/TableRockApp/WorkbenchBridgeConversions.swift`: 331 lines;
- `TableRockApp` directly depends on `TableRockFeature` and `TableRockBridge`;
- app tests use `@testable import TableRock`, reinforcing app-target coupling;
- 17 native feature verifiers search only `TableRockApp.swift`, so extraction
  currently makes otherwise-correct behavior fail structural gates;
- `scripts/build-native-app.sh` already compiles every Swift file in
  `Sources/TableRockApp`, but explicitly enumerates Feature and Bridge objects;
- XcodeGen `native/App/project.yml` and `native/Package.swift` have no
  production presentation module.

Baseline proof already established:

```sh
./scripts/build-native-app.sh

xcodebuild test \
  -project native/App/TableRock.xcodeproj \
  -scheme TableRock \
  -testPlan Checkpoint \
  -destination 'platform=macOS' \
  -derivedDataPath target/native-refactor-baseline-model-testhost-derived-data \
  -only-testing:TableRockFeatureTests \
  -only-testing:TableRockBridgeTests \
  -only-testing:TableRockAppTests \
  CODE_SIGN_IDENTITY=- ENABLE_HARDENED_RUNTIME=NO
```

The direct application build passed. Feature, Bridge, and App suites passed
with 77 tests, four configured live-server skips, and zero failures. The local
`ENABLE_HARDENED_RUNTIME=NO` override only avoids a Team-ID mismatch between the
ad-hoc test host and bundle. Shipping Debug/Release settings remain hardened;
canonical CI still runs its unmodified command. Full production UI baseline was
cancelled during the operator pause and is not claimed.

Before each checkpoint, compare current paths, dependencies, declaration
ownership, tests, and scripts with this plan. Update the plan first if drift
changes a boundary or invalidates a command.

## Final ownership model

Dependency direction is one-way:

```text
TableRockFeature
      ^
      |
TableRockPresentation
      ^
      |
TableRockApp ----------> TableRockBridge ----------> tablerock_ffiFFI
      |                         |
      +-------------------------+----> TableRockFeature

TableRockDesignLab  (no dependencies; evidence only)
```

`TableRockPresentation` never imports `TableRockBridge`,
`TableRockDesignLab`, or `tablerock_ffiFFI`. `TableRockFeature` imports none of
those modules and owns no SwiftUI/AppKit code. The application target is the
only production composition root.

| Responsibility | Final owner and exact path |
|---|---|
| Application entry | `native/Sources/TableRockApp/TableRockApp.swift` |
| Window hosting and restoration | `native/Sources/TableRockApp/ApplicationRoot.swift`, `ApplicationWindows.swift` |
| Runtime composition and backend selection | `native/Sources/TableRockApp/ApplicationRuntime.swift` |
| System/file/keychain/open-panel ports | `native/Sources/TableRockApp/ApplicationPorts.swift` |
| Live backend facade | `native/Sources/TableRockBridge/LiveWorkbenchBackend.swift` |
| Bridge DTO conversions | `native/Sources/TableRockBridge/WorkbenchBridgeConversions.swift` |
| Stable bridge-neutral contracts | `native/Sources/TableRockFeature/WorkbenchTypes.swift`, `AppDependencies.swift`, and focused Feature files created when types are extracted |
| Commands and menus | `native/Sources/TableRockPresentation/WorkbenchCommands.swift` |
| Navigation and workspace state | `native/Sources/TableRockPresentation/WorkbenchPresentationStore.swift`, `WorkbenchPresentationStore+Navigation.swift`, `WorkbenchPresentationStore+Tabs.swift` |
| Toolbar state and composition | `native/Sources/TableRockPresentation/WorkbenchToolbar.swift`, derived from store state |
| Inspector state and surfaces | `native/Sources/TableRockPresentation/WorkbenchInspector.swift`, `ValueInspector.swift`, `StructureInspector.swift` |
| Sheets and alerts | `native/Sources/TableRockPresentation/WorkbenchSheets.swift`, `ConnectionSheets.swift`, `TransferSheets.swift`, `AdministrationSheets.swift` |
| Presentation models and formatters | `native/Sources/TableRockPresentation/PresentationModels.swift`, `PresentationFormatting.swift` |
| Feature workflow coordination | `WorkbenchPresentationStore+Connections.swift`, `+Catalog.swift`, `+Queries.swift`, `+Transfers.swift`, `+Administration.swift`, `+Restoration.swift`; all call bridge-neutral Feature protocols |
| Connection and schema navigation | `native/Sources/TableRockPresentation/ConnectionBrowser.swift`, `CatalogSidebar.swift` |
| Workspace and tabs | `native/Sources/TableRockPresentation/WorkbenchShell.swift`, `WorkspaceTabs.swift`, `QueryWorkspace.swift`, `ObjectWorkspace.swift` |
| PostgreSQL presentation | `native/Sources/TableRockPresentation/PostgreSQLPresentation.swift` |
| ClickHouse presentation | `native/Sources/TableRockPresentation/ClickHousePresentation.swift` |
| Redis presentation | `native/Sources/TableRockPresentation/RedisPresentation.swift` |
| Change review and safety | `native/Sources/TableRockPresentation/ChangeReview.swift`, `SafetyStatus.swift` |
| AppKit catalog adapter | `native/Sources/TableRockPresentation/AppKitCatalogView.swift`; `NSOutlineView` only |
| AppKit result adapter | `native/Sources/TableRockPresentation/AppKitResultGrid.swift`; `NSTableView` only |
| AppKit SQL editor adapter | `native/Sources/TableRockPresentation/AppKitSQLEditor.swift`; `NSTextView` only |
| Reusable native components | `native/Sources/TableRockPresentation/NativeComponents.swift` |
| Liquid Glass helpers | `native/Sources/TableRockPresentation/FunctionalMaterials.swift`; approved functional chrome only |
| Fixtures and scripted backend | `native/Sources/TableRockApp/DevelopmentSupport/ScriptedWorkbenchBackend.swift`, `FixtureRoutes.swift`; development/test compilation only |
| Visual-QA route parsing | `native/Sources/TableRockApp/DevelopmentSupport/VisualQARoutes.swift`; development/test compilation only |
| Visual-QA tests | `native/Tests/TableRockAppUITests/NativeWorkbenchVisualQATests.swift`, existing UI-test files split by surface as needed |
| Presentation unit tests | `native/Tests/TableRockPresentationTests/` |
| App composition tests | `native/Tests/TableRockAppTests/` |
| Design Lab evidence | `native/Sources/TableRockDesignLab/`, `native/Tests/TableRockDesignLabTests/`, `native/Tests/TableRockDesignLabUITests/`; never a production dependency |

Small files are a consequence of coherent ownership, not a line-count game.
No file may reacquire lifecycle, platform I/O, bridge conversion, store,
SwiftUI, and AppKit responsibilities together.

## Common checkpoint protocol

Every implementation checkpoint follows this order:

1. Re-read this package and inspect `git status`, dependency declarations, and
   the exact source/test paths being changed.
2. Record unexpected drift before editing. Stop if operator changes overlap the
   same declarations and cannot be preserved.
3. Add or move tests before changing a behavioral boundary.
4. Make one coherent ownership or visual change. Do not combine unrelated Rust
   behavior, dependencies, or broad cleanup.
5. Run the narrow gate listed for the checkpoint, then
   `./scripts/build-native-app.sh` and affected Xcode test bundles.
6. Review source imports, release membership, generated-project drift, diff,
   and repository status.
7. Commit using Conventional Commits, `git commit -s`, and
   `Co-authored-by: Codex <codex@openai.com>`. Push immediately.
8. If proof fails, repair in the checkpoint or revert it with a new commit.
   Never carry a red checkpoint into the next boundary.

Global stop conditions:

- a change moves database semantics, safety, redaction, credentials, or
  persistence ownership from Rust into Swift;
- Presentation needs to import Bridge, FFI, or Design Lab;
- an approved custom control cannot be built with current macOS 26 APIs;
- clean-room separation cannot be preserved;
- fixture behavior is required in a Release product;
- a gate can pass only by weakening assertions, accessibility, safety, or the
  confirmed design;
- required Xcode/macOS GUI or Accessibility capability is unavailable.

## Checkpoint P0 — freeze architecture package

**Paths**

- `docs/design/native-macos-2026/production-migration-plan.md`
- `docs/evidence/delivery/674-native-production-migration-plan.md`
- `docs/evidence/README.md`

**Prerequisites**

- explicit operator confirmation recorded by evidence 673;
- green direct build and model/bridge/app test baseline;
- no overlapping unexplained worktree changes.

**Drift check**

- confirm the confirmation commit and decision record still identify the same
  refined Native Workbench revision;
- recount source ownership and dependency graph;
- confirm no production target depends on Design Lab.

**Implementation**

- publish this final ownership model and checkpoint sequence;
- classify every open design question as resolved, deferred, or bounded
  research;
- record baseline proof and honest local test-host exception.

**Verification**

```sh
rg -n 'TableRockDesignLab' native/Package.swift native/App/project.yml
./scripts/verify-native-design-lab.sh
git diff --check
```

**Done**

- a zero-context implementer can identify every owner, dependency, command,
  gate, stop condition, and recovery route without reading chat history.

**Stop/recovery**

- stop if approval evidence is absent or contradictory;
- repair documentation forward if repository drift invalidates a path.

## Checkpoint P1 — make gates ownership-aware

**Paths**

- the 17 `scripts/verify-native-*.sh` scripts currently assigning
  `SOURCE=.../TableRockApp.swift`;
- new `scripts/verify-native-source-ownership.sh`;
- `.github/workflows/native.yml` and `native-nightly.yml` only if the new gate
  must be wired there.

**Prerequisites**

- P0 committed and pushed;
- existing scripts pass before modification or their external Docker/GUI
  prerequisite is recorded.

**Drift check**

- use `rg -n '\$SOURCE|^SOURCE=' scripts/verify-native-*.sh`;
- inspect every use, not only declarations;
- confirm none parse or compile a single source file.

**Implementation**

- replace monolith-only source variables with the production source roots
  needed by each assertion;
- keep fixed-string versus regular-expression semantics unchanged;
- add a source-ownership gate that rejects forbidden dependency directions and
  production imports of Design Lab;
- freeze existing Release fixture symbols as explicit debt, reject new symbols
  or spread into more source files, then make the debt set empty in P6;
- add a focused fixture proving a symbol may move files without defeating a
  verifier.

**Verification**

```sh
bash -n scripts/verify-native-*.sh
./scripts/verify-native-source-ownership.sh
./scripts/build-native-app.sh
```

Run structural halves and all available runtime halves of changed verifiers.
No expected pattern may be weakened or deleted.

**Done**

- gates search responsibility roots rather than one filename;
- moving declarations cannot create false failures or hide forbidden imports;
- existing fixture debt cannot grow before its P6 removal;
- direct production build remains green.

**Stop/recovery**

- stop if broadening a root creates ambiguous or false-positive evidence;
- use explicit path arrays per responsibility, then repair forward;
- revert P1 with a new commit if parity with old assertions cannot be proven.

## Checkpoint P2 — extract application integration

**Paths**

- `native/Sources/TableRockApp/TableRockApp.swift`
- new `ApplicationRoot.swift`, `ApplicationWindows.swift`,
  `ApplicationRuntime.swift`, and `ApplicationPorts.swift`
- moved `native/Sources/TableRockBridge/LiveWorkbenchBackend.swift`
- moved `native/Sources/TableRockBridge/WorkbenchBridgeConversions.swift`
- `native/Package.swift`, `native/App/project.yml`,
  `scripts/build-native-app.sh`

**Prerequisites**

- P1 gates accept multi-file ownership;
- baseline application, Feature, Bridge, and App tests pass.

**Drift check**

- inventory all top-level declarations in the lifecycle, ports, backend, and
  conversion regions;
- map imports and access levels before moving code;
- confirm Bridge can depend on Feature without a cycle.

**Implementation**

- move declarations without behavior or naming changes;
- make `TableRockBridge` depend on `TableRockFeature` and compile bridge
  conversions/backend facade there;
- keep `TableRockApp.swift` as the small `@main` entry only;
- update direct Swift compilation and Xcode/SwiftPM graphs;
- keep blocking bridge calls outside all view `body` paths.

**Verification**

```sh
./scripts/generate-swift-bindings.sh
./scripts/build-native-app.sh
swift test --package-path native -c release
xcodebuild test -project native/App/TableRock.xcodeproj -scheme TableRock \
  -testPlan Checkpoint -destination 'platform=macOS' \
  -derivedDataPath target/native-p2-derived-data \
  -only-testing:TableRockFeatureTests \
  -only-testing:TableRockBridgeTests \
  -only-testing:TableRockAppTests CODE_SIGN_IDENTITY=-
```

The local Team-ID workaround may add `ENABLE_HARDENED_RUNTIME=NO` for tests
only; CI and Release settings remain unchanged.

**Done**

- application entry owns no backend implementation or conversion code;
- Bridge owns FFI translation and exposes bridge-neutral Feature contracts;
- direct and generated project builds agree.

**Stop/recovery**

- stop on a dependency cycle or required public exposure of generated FFI
  types;
- restore declarations with a forward revert if the boundary cannot remain
  bridge-neutral, then record bounded design research.

## Checkpoint P3 — extract presentation state within app target

**Paths**

- `native/Sources/TableRockApp/PresentationModels.swift`
- `native/Sources/TableRockApp/WorkbenchPresentationStore.swift`
- domain store extensions: `+Connections.swift`, `+Catalog.swift`,
  `+Navigation.swift`, `+Tabs.swift`, `+Queries.swift`, `+Transfers.swift`,
  `+Administration.swift`, `+Restoration.swift`
- `native/Tests/TableRockAppTests/`

**Prerequisites**

- P2 green;
- tests cover current connection, tab, query, restoration, change-review,
  export/import, and inspector state transitions.

**Drift check**

- inventory every stored property and method on `BridgeModel`;
- classify stored state as authoritative, derived, transient presentation, or
  external handle;
- detect detached tasks, blocking work, and duplicated derived state.

**Implementation**

- rename `BridgeModel` to `WorkbenchPresentationStore` only after call sites
  are moved mechanically;
- split extensions by workflow without changing behavior;
- keep the store `@MainActor` with the current stable Observation model;
- keep derived state computed and stable identities explicit;
- isolate structured tasks and cancellation ownership; do not add unowned
  detached tasks;
- type errors crossing into presentation.

**Verification**

```sh
./scripts/build-native-app.sh
xcodebuild test -project native/App/TableRock.xcodeproj -scheme TableRock \
  -testPlan Checkpoint -destination 'platform=macOS' \
  -derivedDataPath target/native-p3-derived-data \
  -only-testing:TableRockAppTests CODE_SIGN_IDENTITY=-
./scripts/verify-native-query-tabs.sh
./scripts/verify-native-object-tabs.sh
./scripts/verify-native-history.sh
./scripts/verify-native-saved-queries.sh
./scripts/verify-native-sql-files.sh
```

**Done**

- one named store owns mutable presentation state;
- no view owns bridge handles or database workflow state;
- no integration or platform implementation remains in the store files;
- behavior and fixture proofs remain unchanged.

**Stop/recovery**

- stop if extraction changes cancellation, restoration, safety, or mutation
  semantics;
- restore the affected domain extension with a forward revert, add missing
  characterization tests, then retry.

## Checkpoint P4 — extract SwiftUI and AppKit presentation

**Paths**

- current presentation declarations in `TableRockApp.swift`
- app-target versions of final Presentation files listed in the ownership map;
- `native/Tests/TableRockAppUITests/`

**Prerequisites**

- P3 green store boundary;
- app UI tests can launch deterministic routes.

**Drift check**

- map every SwiftUI view to one surface owner;
- map each AppKit coordinator callback to a typed store intent;
- identify platform I/O or bridge calls reachable from `body`.

**Implementation**

- move views by complete surface: shell, connections, query, object, review,
  sheets, inspector, toolbar, then reusable components;
- move AppKit adapters into their three narrow files;
- adapters own native selection, scrolling, editing, resizing/reordering,
  focus, accessibility, and context-menu translation only;
- remove work from `body` and preserve view identity and focus bindings;
- do not change layout or materials yet.

**Verification**

```sh
./scripts/build-native-app.sh
./scripts/verify-native-accessibility.sh
./scripts/verify-native-value-inspector.sh
./scripts/verify-native-result-copy.sh
xcodebuild test -project native/App/TableRock.xcodeproj -scheme TableRock \
  -testPlan Checkpoint -destination 'platform=macOS' \
  -derivedDataPath target/native-p4-derived-data CODE_SIGN_IDENTITY=-
```

**Done**

- `TableRockApp.swift` contains only entry/composition;
- SwiftUI surfaces and AppKit adapters have separate owners;
- AppKit callbacks send typed intents and contain no business behavior;
- complete Checkpoint plan passes.

**Stop/recovery**

- stop on focus, selection, accessibility-tree, or stable-identity regression;
- revert only the affected surface with a forward commit and retain completed
  surface extractions.

## Checkpoint P5 — enforce the Presentation module

**Paths**

- new `native/Sources/TableRockPresentation/`
- new `native/Tests/TableRockPresentationTests/`
- `native/Package.swift`
- `native/App/project.yml`
- generated `native/App/TableRock.xcodeproj/`
- `scripts/build-native-app.sh`
- `scripts/verify-native-source-ownership.sh`

**Prerequisites**

- P4 files compile inside the app target with clear imports;
- no presentation file requires Bridge or app-only platform ports.

**Drift check**

- calculate the proposed graph from SwiftPM and XcodeGen;
- inspect all presentation imports and public symbols;
- inventory App tests that should become Presentation tests.

**Implementation**

- add `TableRockPresentation` library/static-framework target depending only
  on `TableRockFeature`;
- move store, SwiftUI, and AppKit files to that target;
- expose one narrow root surface plus explicit command/window hooks;
- move presentation tests and keep application composition tests in App tests;
- update direct build to compile Presentation independently;
- make source-ownership gate reject Bridge, FFI, Design Lab, networking,
  persistence, and credential imports in Presentation.

**Verification**

```sh
xcodegen generate --spec native/App/project.yml
swift package dump-package --package-path native
swift test --package-path native -c release
./scripts/verify-native-source-ownership.sh
./scripts/build-native-app.sh
xcodebuild test -project native/App/TableRock.xcodeproj -scheme TableRock \
  -testPlan Checkpoint -destination 'platform=macOS' \
  -derivedDataPath target/native-p5-derived-data CODE_SIGN_IDENTITY=-
```

**Done**

- compiler-enforced dependency graph matches the diagram;
- Presentation tests no longer import the app target;
- Design Lab remains dependency-free;
- app target is composition/integration, not a presentation monolith.

**Stop/recovery**

- stop if enforcing the module requires exposing FFI types or moving platform
  I/O into Presentation;
- forward-revert target creation while keeping coherent file extraction, then
  narrow protocols before retrying.

## Checkpoint P6 — remove fixtures from Release paths

**Paths**

- `native/Sources/TableRockApp/DevelopmentSupport/`
- `native/App/project.yml`
- `native/Package.swift`
- `native/Tests/TableRockAppTests/`
- `native/Tests/TableRockAppUITests/`
- `scripts/verify-native-source-ownership.sh`

**Prerequisites**

- P5 module graph green;
- every fixture environment route has an owning test.

**Drift check**

- enumerate `TABLEROCK_FIXTURE_`, scripted backend, preview, and audit symbols;
- inspect Release compilation and executable strings before moving them.

**Implementation**

- move scripted backend, invented fixtures, and visual-QA route parsing into
  `DevelopmentSupport`;
- compile the directory only for Debug/Test Release or guard it with an
  auditable development compilation condition;
- keep Release startup wired only to the live facade;
- preserve deterministic test launch routes without reading Design Lab data.

**Verification**

```sh
./scripts/build-native-app.sh
xcodebuild archive -project native/App/TableRock.xcodeproj -scheme TableRock \
  -configuration Release -destination 'generic/platform=macOS' \
  -archivePath target/TableRock-p6.xcarchive CODE_SIGN_IDENTITY=-
./scripts/verify-native-source-ownership.sh
strings target/TableRock-p6.xcarchive/Products/Applications/TableRock.app/Contents/MacOS/TableRock \
  | if rg 'TABLEROCK_FIXTURE_|ScriptedWorkbenchBackend'; then exit 1; fi
```

The negative Release check must produce no matches. Run all fixture-owned UI
tests after it.

**Done**

- shipping Release contains no scripted backend or fixture route;
- Debug/Test deterministic routes remain isolated and passing;
- production never consumes Design Lab fixtures.

**Stop/recovery**

- stop if a production workflow depends on scripted state;
- move only the missing test seam behind a protocol, never keep the fixture in
  Release to make tests easier.

## Checkpoint P7 — implement approved shell, windows, commands, and toolbar

**Paths**

- `ApplicationRoot.swift`, `ApplicationWindows.swift`
- `TableRockPresentation/WorkbenchShell.swift`
- `WorkbenchCommands.swift`, `WorkbenchToolbar.swift`,
  `FunctionalMaterials.swift`, `NativeComponents.swift`
- affected Presentation and UI tests

**Prerequisites**

- P6 green architecture and fixture separation;
- confirmed captures for typical/minimum/expanded windows available.

**Drift check**

- compare current production shell behavior with experience brief,
  information architecture, component-material map, and confirmed captures;
- preserve existing multi-window/restoration semantics.

**Implementation**

- implement system `NavigationSplitView` workbench hierarchy;
- implement application windows, restoration, system toolbar, menus, and
  standard keyboard commands;
- derive toolbar enablement from store state;
- use Liquid Glass only through system functional chrome and approved helper;
- keep workbench content opaque.

**Verification**

```sh
./scripts/build-native-app.sh
./scripts/verify-native-multi-window.sh
./scripts/verify-native-accessibility.sh
xcodebuild test -project native/App/TableRock.xcodeproj -scheme TableRock \
  -testPlan Checkpoint -destination 'platform=macOS' \
  -derivedDataPath target/native-p7-derived-data CODE_SIGN_IDENTITY=-
```

Capture real windows at minimum, typical, expanded, active, and inactive
states. Compare composition, not TablePro pixels.

**Done**

- production hierarchy matches confirmed Native Workbench;
- commands, focus, restoration, resizing, and window identity pass;
- no glass appears on editor/data/inspector content.

**Stop/recovery**

- stop if system controls cannot meet approved behavior without an unapproved
  custom component;
- preserve old shell until full replacement route passes, then contract in a
  later green commit.

## Checkpoint P8 — implement connections and persistent catalog

**Paths**

- `ConnectionBrowser.swift`, `ConnectionSheets.swift`, `CatalogSidebar.swift`
- `AppKitCatalogView.swift`
- store connection/catalog extensions
- engine presentation files

**Prerequisites**

- P7 shell owns stable leading navigation and context placement;
- current profile/group/editor/catalog behavior is characterized.

**Drift check**

- inventory profile groups, search, favorites, engine/environment/safety
  labels, catalog refresh, expansion, and empty/loading/error behavior.

**Implementation**

- migrate connection list and setup sheet to confirmed density/hierarchy;
- implement persistent catalog with compact context strip;
- preserve native outline selection, disclosure, keyboard operation, and
  accessibility;
- project PostgreSQL, ClickHouse, and Redis capabilities explicitly; show
  unsupported capabilities rather than hiding them.

**Verification**

```sh
./scripts/verify-native-profile-editor.sh
./scripts/verify-native-profile-groups.sh
./scripts/verify-native-accessibility.sh
./scripts/build-native-app.sh
```

Add UI routes for all three engines plus empty, loading, connection-error, and
long-identifier states.

**Done**

- connection/setup/catalog surfaces match confirmed structure;
- selection, search, refresh, focus, and three-engine capability truth pass;
- no credential or resolved 1Password value enters presentation logs/state.

**Stop/recovery**

- stop on credential exposure, capability ambiguity, or catalog selection
  regression;
- keep old production surface reachable only until the new route passes, then
  remove it in the same checkpoint.

## Checkpoint P9 — implement tabs, query/editor, results, and history

**Paths**

- `WorkspaceTabs.swift`, `QueryWorkspace.swift`, `ObjectWorkspace.swift`
- `AppKitSQLEditor.swift`, `AppKitResultGrid.swift`
- store navigation/tab/query extensions
- history/saved-query/SQL-file presentation files if split further

**Prerequisites**

- P8 context and catalog selection stable;
- current close/cancel/dirty/restoration and result-copy behavior covered.

**Drift check**

- inventory query/object tab limits, preview/pinned/dirty/running states,
  editor file conflict behavior, result paging/copy/export, history, and saved
  queries.

**Implementation**

- implement confirmed document-style tabs and query/result composition;
- preserve `NSTextView` editing/IME/find behavior and `NSTableView` density,
  column behavior, selection, paging, context menus, and accessibility;
- keep query/cancel/export bridge work outside rendering;
- retain safety status continuously in context/chrome.

**Verification**

```sh
./scripts/verify-native-query-tabs.sh
./scripts/verify-native-object-tabs.sh
./scripts/verify-native-history.sh
./scripts/verify-native-saved-queries.sh
./scripts/verify-native-sql-files.sh
./scripts/verify-native-result-copy.sh
./scripts/verify-native-performance.sh target/native-p9-performance
```

Run complete Checkpoint Xcode plan and real-window captures for SQL/results,
large-result, long-identifiers, loading, cancel, and error states.

**Done**

- tab/query/result workflow matches confirmed Native Workbench;
- close, dirty, running, restoration, copy/export, keyboard, IME, and
  performance behavior remains correct;
- data/editor content is opaque.

**Stop/recovery**

- stop on cancellation truth, data truncation, export, focus, IME, or
  performance regression;
- revert the affected adapter/surface forward without rolling back completed
  shell/catalog checkpoints.

## Checkpoint P10 — implement data, structure, and inspectors

**Paths**

- `ObjectWorkspace.swift`, `ValueInspector.swift`, `StructureInspector.swift`,
  `WorkbenchInspector.swift`
- `AppKitResultGrid.swift`
- PostgreSQL/ClickHouse/Redis presentation files
- affected store and tests

**Prerequisites**

- P9 stable grid selection and object tabs;
- typed PageV1 and bounded structured-value contracts remain unchanged.

**Drift check**

- inventory typed cell states, truncation, NULL/binary/JSON presentation,
  structure capabilities, inspector selection, collapse/resize, and engine
  differences.

**Implementation**

- migrate data and structure layouts plus trailing inspector;
- derive inspector state from current selection; do not duplicate selected
  value ownership;
- preserve bounded JSON tree, hex/text, database type, truncation truth, and
  accessible labels;
- use opaque inspector/content backgrounds and standard split behavior.

**Verification**

```sh
./scripts/verify-native-value-inspector.sh
./scripts/verify-native-structure.sh
./scripts/verify-native-clickhouse-structure.sh
./scripts/verify-native-redis-key-view.sh
./scripts/verify-native-redis-overview.sh
./scripts/verify-native-page-performance.sh
./scripts/build-native-app.sh
```

**Done**

- selected-cell, value, structure, engine-specific, large-result, and empty
  inspector states match approved hierarchy;
- typed/truncated/binary values remain truthful and bounded;
- inspector keyboard/focus/accessibility behavior passes.

**Stop/recovery**

- stop on value-truth, bounds, engine-capability, or selection regression;
- forward-revert only affected inspector/object surface and add missing typed
  characterization proof.

## Checkpoint P11 — implement review, safety, administration, and all states

**Paths**

- `ChangeReview.swift`, `SafetyStatus.swift`, `AdministrationSheets.swift`,
  `TransferSheets.swift`
- store transfer/administration extensions
- engine presentation files
- affected tests and verifier scripts

**Prerequisites**

- P10 stable workspace and inspector composition;
- review/apply tokens and Rust safety authority unchanged.

**Drift check**

- inventory staged changes, reviewed apply, destructive confirmation,
  maintenance, CSV import/export, roles/tools, unsupported engine paths,
  progress/cancel/error/partial/ambiguous outcomes.

**Implementation**

- migrate change-review and continuously visible safety surfaces;
- migrate administration and transfer sheets without changing Rust intents;
- render empty, loading, disconnected, permission, timeout, cancellation,
  partial, ambiguous, stale-review, and unsupported states explicitly;
- preserve typed destructive confirmation and one-use review authority.

**Verification**

```sh
./scripts/verify-native-csv-import.sh
./scripts/verify-native-maintenance.sh
./scripts/verify-native-structure.sh
./scripts/verify-native-clickhouse-structure.sh
./scripts/verify-native-redis-key-view.sh
./scripts/verify-native-redis-overview.sh
./scripts/verify-native-behavior.sh
./scripts/build-native-app.sh
```

Run complete Checkpoint Xcode plan plus real three-engine fixture routes.

**Done**

- review/safety behavior matches approved design without weakening Rust
  authority;
- all required state classes have explicit non-color presentation;
- administration/transfer workflows retain progress, cancel, and failure truth.

**Stop/recovery**

- stop on any weakened confirmation, review-token, ambiguous-write,
  cancellation, redaction, or credential invariant;
- retain existing safe behavior and forward-revert visual composition if the
  approved design cannot express it truthfully.

## Checkpoint P12 — contract old presentation and dead paths

**Paths**

- all `native/Sources/TableRockApp/` and `TableRockPresentation/` files
- `native/Tests/TableRockAppTests/`, `TableRockPresentationTests/`, and UI tests
- `native/Package.swift`, `native/App/project.yml`, generated Xcode project
- source-ownership and feature verifiers

**Prerequisites**

- P7–P11 replacement surfaces pass in production;
- no test reaches superseded views or fixture implementation indirectly.

**Drift check**

- use compiler references and `rg` to inventory superseded types, routes,
  duplicated state, compatibility branches, and dead adapters;
- compare Release dependency graph and executable symbols with P6.

**Implementation**

- delete superseded production presentation and duplicate state;
- delete obsolete scripted/fixture paths rather than leaving disabled code;
- remove compatibility branches below macOS 26;
- keep Design Lab as historical comparison/approved evidence, not a source
  dependency;
- enforce file/module ownership in CI.

**Verification**

```sh
./scripts/verify-native-source-ownership.sh
swift test --package-path native -c release
./scripts/build-native-app.sh
xcodebuild test -project native/App/TableRock.xcodeproj -scheme TableRock \
  -testPlan Checkpoint -destination 'platform=macOS' \
  -derivedDataPath target/native-p12-derived-data CODE_SIGN_IDENTITY=-
```

Run every native feature verifier. Confirm Release contains no fixture symbols
and Presentation has no forbidden imports.

**Done**

- monolith enabling condition is absent at file and compiler dependency levels;
- old presentation, duplicate state, and release fixture behavior are gone;
- every retained declaration has one named owner and active coverage.

**Stop/recovery**

- stop if deletion reveals an unimplemented production behavior;
- restore only that behavior through the correct new owner, never restore the
  monolith or create a permanent parallel path.

## Checkpoint P13 — final Liquid Glass remediation

**Paths**

- all production SwiftUI/AppKit presentation files;
- `FunctionalMaterials.swift`;
- visual-QA tests and evidence manifests.

**Prerequisites**

- P12 single production presentation path;
- real production window capture capability.

**Drift check**

- enumerate every material, blur, translucency, tint, shadow, border, custom
  toolbar/sidebar, and custom control;
- classify each region as CONTENT or FUNCTIONAL using the approved map.

**Implementation**

- remove glass on content, glass-on-glass, decorative glass, arbitrary blur,
  simulated materials, excess shadows/borders/tint, and hand-painted system
  chrome;
- replace custom controls with standard components where behavior permits;
- retain custom glass only when functional, approved, necessary, and proven
  across accessibility states.

**Verification**

- capture actual production windows by window ID in light/dark, active/inactive,
  Reduce Transparency, Increase Contrast, and Reduce Motion;
- run accessibility and complete Checkpoint plans;
- inspect captures for stacking, content transparency, contrast, and window
  activation behavior.

**Done**

- every remaining material has recorded functional justification;
- no content glass, stacked glass, arbitrary blur, or handcrafted system
  imitation remains;
- runtime accessibility variants remain legible and correct.

**Stop/recovery**

- stop if GUI session or accessibility settings cannot be controlled and
  restored;
- do not substitute detached component snapshots for real chrome evidence.

## Checkpoint P14 — production visual and accessibility QA

**Paths**

- `native/Tests/TableRockAppUITests/`
- production capture script(s) under `scripts/`
- `docs/evidence/native-production/native-workbench/`
- approved design contracts and product docs when observed behavior changes.

**Prerequisites**

- P13 material audit green;
- Screen Recording and Accessibility permissions available to the test session.

**Drift check**

- regenerate route/state inventory from production tests;
- compare against all dimensions in `design-contracts.md` and actual production
  workflows;
- do not use TablePro pixels as regression references.

**Implementation**

- run deterministic production routes where real servers cannot create a state
  repeatably; use real three-engine behavior wherever possible;
- capture the running window by window ID at minimum, typical, and expanded
  sizes;
- cover navigation, toolbar, tables, inspectors, sheets, menus, commands,
  query/results, review/safety, all engines, light/dark, active/inactive,
  accessibility variants, keyboard-only operation, and accessibility actions;
- execute `performAccessibilityAudit` and pixel regression against the
  operator-confirmed Native Workbench composition.

**Verification**

- all XCUITests and accessibility audits exit zero;
- capture manifest records revision, toolchain, OS, arguments, window ID,
  dimensions, and checksums;
- system appearance/accessibility settings are restored and verified.

**Done**

- complete production matrix has real-window evidence;
- no hard visual, interaction, keyboard, focus, resize, menu, sheet, table,
  inspector, material, or accessibility failure remains.

**Stop/recovery**

- stop and record exact permission/session/toolchain blocker when real-window
  proof cannot run;
- never mark an unexecuted or detached-only chrome check as passed.

## Checkpoint P15 — final design review and repository readiness

**Paths**

- architecture/product/user docs affected by final ownership and behavior;
- `ROADMAP.md` and new evidence documents;
- all source, tests, scripts, workflows, and generated project files changed by
  P0–P14.

**Prerequisites**

- P14 complete with no visual/accessibility hard failure;
- all local and external blockers retested.

**Drift check**

- reconcile production only against the experience brief, operator decision,
  component/material map, confirmed Design Lab runtime/captures, production
  captures, and actual behavior;
- inventory current CI and repository commands again before final execution.

**Implementation**

- correct hard failures in source-owner order, then rerun material,
  accessibility, visual, and design review;
- document final boundaries, Design Lab operation, production build, visual QA,
  evidence locations, macOS 26 minimum, SwiftUI-first/AppKit exceptions,
  CONTENT/FUNCTIONAL rules, clean-room provenance, and obsolete-authority
  removal;
- update roadmap/evidence only for proof actually completed.

**Final verification categories**

Run commands discovered from current workflows, including at least:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check --workspace --all-targets --locked
cargo nextest run -p tablerock-core -p tablerock-persistence \
  -p tablerock-tui -p tablerock-cli -p tablerock-ffi --locked
cargo nextest run -p tablerock-engine --locked
cargo audit
cargo deny check
cargo outdated --workspace --root-deps-only --exit-code 1

cargo build -p tablerock-ffi --release --locked
./scripts/generate-swift-bindings.sh
swift test --package-path native -c release
SKIP_BINDINGS=1 ./scripts/build-xcframework.sh
./scripts/build-native-app.sh
./scripts/verify-native-design-lab.sh
./scripts/verify-native-source-ownership.sh

xcodebuild test -project native/App/TableRock.xcodeproj -scheme TableRock \
  -testPlan Checkpoint -destination 'platform=macOS' \
  -derivedDataPath target/native-final-derived-data CODE_SIGN_IDENTITY=-
```

Also run all native verifier scripts, production visual/accessibility matrix,
documentation checks found in CI, Release archive, clean status, and relevant
real-server/nightly/release gates. Signing/notarization commands run only when
operator credentials are available; otherwise their existing gate stays
honestly blocked.

**Done**

- production monolith enabling architecture is removed;
- production matches the confirmed Native Workbench;
- Rust/domain/safety ownership and all three engines remain correct;
- SwiftUI is primary and AppKit boundaries are narrow and justified;
- final materials, accessibility, visual, design, and repository gates pass;
- no unexplained worktree changes or `TODO`/`IN PROGRESS` task remains;
- every coherent checkpoint is signed, carries the required co-author trailer,
  and is pushed.

**Stop/recovery**

- stop on any honest failing gate or unavailable required machine capability;
- preserve and push only coherent green checkpoints;
- record exact command, environment, shortest decisive failure, affected done
  criterion, and safe next action.

## Clean-room provenance

Public TablePro pages and documentation informed broad workbench organization:
persistent database context, dense native result presentation, query tabs,
inspectors, connection sheets, and keyboard-first flow. No TablePro source,
tests, comments, bundle internals, branding, assets, product copy, or
proprietary fixtures were inspected, copied, translated, or used.

Apple macOS behavior and the confirmed TableRock Native Workbench are the final
design authorities. The rejected Sketch is not an authority and must not be
reintroduced.
