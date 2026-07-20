# `/goal` Prompt: Complete TableRock CLI/TUI and Native macOS App

## Goal

Implement the entire TableRock program defined by **every plan in `plans/`**.
Finish the production CLI/TUI and full native macOS application for PostgreSQL,
ClickHouse, and Redis, then finish distribution and closure. Every plan file is
an execution contract: complete every step, checkbox, command, artifact,
behavior, test, evidence requirement, maintenance-triggered update, and done
criterion. Do not treat a summary, status row, roadmap phase, vertical slice,
preview, prototype, partially working UI, "mostly complete" phase, or narrow
green test as a substitute for executing the underlying plans. Persist across
sessions until every plan is fully implemented, verified against current state,
and honestly marked `DONE`, or a proven external STOP condition requires one
specific operator action.

This invocation approves the repository's fixed product and architecture
decisions and authorizes all remaining implementation required by every file
currently present in `plans/`, including plans 001–022 and any new plan needed
to close a discovered gap. Existing `DONE` labels are evidence claims, not
authority to skip reading or verification. Read every plan fully. Reopen and
repair any completed plan whose current code, tests, documentation, runtime
behavior, UI, artifacts, or remote evidence does not satisfy its full contract.

## Plans are the execution backbone

`plans/README.md` and every numbered Markdown file under `plans/` define the
dependency-ordered implementation backlog. The primary job is to execute that
backlog completely, not merely audit or rewrite it.

The required plan inventory is:

1. `001-ci-verification-baseline.md` — continuously verified build/test/lint CI.
2. `002-engine-sessions-and-arbitrary-queries.md` — persistent engine sessions
   and arbitrary-statement execution.
3. `003-catalog-listing-service.md` — PostgreSQL, ClickHouse, and Redis catalog
   listing behind the shared service.
4. `004-profile-domain-and-secret-resolution.md` — complete profile domain,
   grouping/search, environment tags, and secret resolution.
5. `005-tui-effect-executor-and-engine-bridge.md` — TUI effects, engine and
   persistence bridge, and screen-submodel architecture.
6. `006-connection-experience-screens.md` — complete connection list, groups,
   editor, Test, and Connect experience.
7. `007-workbench-shell-and-session-lifecycle.md` — context bar, catalog, tabs,
   status, sessions, and context lifecycle.
8. `008-termrock-virtualgrid.md` — reusable TermRock `VirtualGrid` and exact
   TableRock pin.
9. `009-postgresql-read-only-slice.md` — PostgreSQL grid, SQL streaming,
   inspector, and Phase 4 exit.
10. `010-termrock-textarea-completionmenu.md` — complete TermRock `TextArea`
    contract and `CompletionMenu`.
11. `011-sql-editor-workbench.md` — multiline SQL editor, statement selection,
    completion, history, saved queries, and files.
12. `012-grid-controls-and-copy.md` — server sorting, filters, columns, and all
    copy formats.
13. `013-postgresql-writes-and-admin.md` — staged edits, review, transactional
    apply, conflict truth, and PostgreSQL administration.
14. `014-clickhouse-slice.md` — complete ClickHouse engine and UI slice.
15. `015-redis-slice.md` — complete Redis engine and UI slice.
16. `016-daily-workflows-and-data-movement.md` — result tabs, import/export,
    preferences, files, and resilience.
17. `017-scoped-parity-expansion.md` — SSH, pg_dump/restore, DDL, roles, and
    editor/Vim polish.
18. `018-tui-hardening-release-gate.md` — complete TUI hardening and parity
    release gate.
19. `019-uniffi-bridge-proof.md` — UniFFI bridge, XCFramework, Developer ID,
    hardened runtime, notarization, stapling, and clean-machine proof.
20. `020-native-macos-vertical-slice.md` — native SwiftUI/AppKit Liquid Glass
    vertical slice over the Rust bridge.
21. `021-native-parity-and-closure.md` — every native workflow, release
    evidence, accessibility/performance, and Phases 14–15 closure.
22. `022-preview-cicd-and-homebrew-tap.md` — rolling attested preview, CLI/TUI
    formula, native cask, pull verification, and real installation lifecycle.

This list is a minimum, not a frozen ceiling. At startup, compare it with the
live `plans/` directory and add any newer numbered plans to the execution
matrix. A listed plan remains required even when its index row already says
`DONE`.

For every plan, in dependency order:

1. Read the entire current plan, including drift checks, current-state claims,
   scope, exclusions, exact steps, commands, test plan, done criteria, STOP
   conditions, and maintenance notes.
2. Validate its assumptions against live code, current upstream documentation,
   installed/latest stable tools, CI, artifacts, and remote repositories.
3. Amend stale or incomplete plan text before dependent implementation. If a
   required behavior or screen belongs to no existing plan, create the next
   numbered dependency-linked plan with concrete steps and evidence gates.
4. Execute every step. Do not skip a step because similar code exists; inspect
   and prove that existing code meets the exact current requirement.
5. Run every named command and inspect its output. Add missing tests when the
   named commands do not cover the plan's full claim.
6. Satisfy every done checkbox with direct evidence, add/index required evidence
   documents, then change the plan's row to `DONE` in the same checkpoint.
7. Continue immediately to the next dependency-ready plan. Never stop after
   planning, scaffolding, one screen, one client, or one passing test class.

No requirement may disappear between a plan, roadmap, product document,
parity-ledger row, implementation, test, and evidence. When they disagree,
resolve the mismatch explicitly and strengthen the owning plan.

## Product outcome

Deliver one complete database workbench through two first-class clients over
the same Rust-owned behavior:

- `tablerock`: a complete Rust CLI/TUI using one root The Elm Architecture
  flow, TermRock, Ratatui, and Crossterm.
- `TableRock.app`: a complete native macOS app using the newest stable macOS,
  Xcode, Swift language mode, SwiftUI, AppKit, and platform design APIs
  available when each checkpoint executes.
- Shared Rust services for PostgreSQL, ClickHouse, Redis, persistence, safety,
  paging, editing, history, import/export, diagnostics, and release identity.
- Installable, verified release artifacts and Homebrew formula/cask delivery
  required by plan 022.

Both clients must implement the full in-scope functional-parity ledger. Native
macOS is not a wrapper around the TUI and must not use a WebView, daemon, local
RPC, or manual C ABI.

## Authority

Before changing code, read `AGENTS.md`, `CONTRIBUTING.md`, `README.md`,
`ROADMAP.md`, `plans/README.md`, every numbered plan in full, and all
documents linked from `docs/README.md` that govern the current checkpoint.
Always include these authorities:

- `docs/product/` for screen and workflow behavior.
- `docs/architecture/functional-parity-ledger.md` for feature scope.
- `docs/architecture/application-pattern.md` and
  `docs/architecture/rust-core-architecture.md` for ownership boundaries.
- `docs/architecture/native-macos-path.md` and
  `docs/product/native-macos.md` for native design.
- `docs/architecture/dependency-policy.md` for latest-stable adoption.
- `docs/architecture/delivery-plan.md` and
  `docs/architecture/quality-and-verification.md` for execution and evidence.
- `docs/architecture/main-branch-delivery.md` for delivery.

Resolve conflicts in this order: `AGENTS.md`, fixed architecture decisions,
product requirements, roadmap, dependency-ordered plans, then older evidence.
Live code, current upstream behavior, and direct tests override stale snapshots.
Repair stale plans and docs in the same checkpoint; never silently narrow the
goal to match existing code.

Use Context7 for current library/framework/SDK/API/CLI documentation when
available. For Apple platform APIs and tooling, use current primary Apple
documentation and current Xcode SDK interfaces. For database behavior, use
official database/client documentation and direct tests. Apply the clean-room
rule and record provenance for externally influenced work.

## TablePro reference rule

Use TablePro as the primary external product reference throughout discovery,
planning, implementation review, and final parity review. At the start of each
screen family, inspect TablePro's current public user-facing documentation,
public feature descriptions, and high-level public screenshots to identify the
broad workflows, states, and operator expectations that a complete database
workbench must cover. Re-check it during final screen audit so obvious workflow
classes are not missed.

TablePro is a reference for **what problems and broad workflows exist**, never
an implementation or visual-expression source. Its AGPL source must not be read
for implementation guidance or copied, translated, or structurally ported.
Never copy its source, tests, comments, identifiers, strings, assets, icons,
colors, geometry, layout measurements, key bindings, or distinctive screen
expression. Derive every TableRock requirement independently in
`docs/product/`, implement it from official platform/database/library docs,
and verify it with TableRock-owned tests. Every TablePro-influenced checkpoint
records the required clean-room provenance block. If TablePro and TableRock
architecture conflict, TablePro establishes only the workflow need;
TableRock's fixed architecture controls the solution.

## Fixed architecture

Rust owns all database, persistence, safety, redaction, paging, mutation,
history, import/export, cancellation, and diagnostics behavior. Keep database
client types behind adapters and pass bounded owned pages across stable
contracts.

The CLI/TUI uses one root TEA
Model/Message/Update/Effect/Subscription/View flow. TermRock owns terminal
lifecycle and reusable controls; Ratatui renders; Crossterm 0.29 is the only
terminal backend/input. Keep I/O out of update and render paths. Add missing
neutral reusable primitives to TermRock first, test and document them there,
push TermRock `main`, then pin the exact revision here. Jackin remains read-only.

The macOS app embeds Rust through coarse synchronous UniFFI. Swift owns
presentation and OS integration only. Use SwiftUI for app/window/commands,
navigation, settings, and modern platform composition; use AppKit-backed
`NSOutlineView`, `NSTableView`, and `NSTextView` where data density, editing,
IME, focus, or accessibility requires them. Preserve strict Swift concurrency,
`@MainActor` presentation ownership, bounded page transfer, native
accessibility, restoration, multi-window isolation, and cancellation truth.

## Latest-platform policy

Optimize for the newest stable macOS ecosystem, not backward compatibility.
At each native checkpoint:

1. Discover and record the latest stable macOS, Xcode, Swift, SwiftUI/AppKit
   API surface, runner image, and relevant dependency releases.
2. Set the deployment target to the newest stable macOS supported by the
   selected stable Xcode unless a repository requirement explicitly demands a
   newer beta. Do not retain fallback UI, availability branches, compatibility
   shims, legacy materials, deprecated APIs, or old deployment targets merely
   for older systems.
3. Prefer current native design and lifecycle APIs. Use Liquid Glass and its
   current successor guidance exactly as provided by the selected stable SDK.
4. Upgrade stale dependencies and tooling immediately unless a direct test
   proves an upstream constraint. Record any temporary constraint and its
   removal trigger.
5. Update architecture, product, plans, CI runners, tests, and evidence when
   the current stable platform makes an older assumption stale.

"Latest" never permits speculative private APIs or an unstable beta when a
stable release exists. Correctness, platform consistency, and goal fit decide;
compatibility cost does not justify preserving known-old design.

## Execution order

Work directly on `main` through small forward-only checkpoints. Start with a
current-state audit, not status labels:

1. Verify worktree ownership, `HEAD == origin/main`, remote CI, toolchain, and
   plan drift. Preserve unrelated user changes.
2. Build a requirement matrix for every file under `plans/`, roadmap Phases
   0–15, every
   applicable parity-ledger row, and every named deliverable/test/evidence
   gate. Map each requirement to authoritative current evidence.
3. Classify each item as proven complete, contradicted, incomplete, weakly
   evidenced, or missing. Anything except proven complete remains work.
4. Execute plans in dependency order. Prioritize unresolved defects in earlier
   plans before dependent work; then finish plan 019's remaining distribution
   gate, all of plan 021, plan 022, and any reopened earlier work.
5. Complete CLI/TUI parity and hardening before claiming native parity.
6. Complete native macOS parity, real-server behavior, accessibility,
   performance, restoration, packaging, signing/notarization, clean-machine
   installation, and uninstall evidence.
7. Complete rolling preview distribution and pull-verified Homebrew formula
   and cask delivery.
8. Perform final cross-plan audit, close every ledger row, update every status,
   and prove clean synchronized repositories and published state.

## Iterative planning and screen closure

Current plans are a starting execution graph, not proof that their combined
scope is complete. Keep iterating this prompt, roadmap, product specification,
parity ledger, and plans until they fully describe and verify the product:

1. Before implementation, derive a canonical screen manifest from
   `docs/product/`, the parity ledger, plans 001–022, current application
   behavior, and TablePro's allowed public workflow evidence.
2. Give every screen, window, tab type, sidebar/panel, toolbar, dialog, sheet,
   popover, menu/command, inspector, editor, browser, settings surface,
   import/export flow, review flow, and engine-specific view a stable
   TableRock-owned requirement ID.
3. For each surface, enumerate both-client applicability, engine applicability,
   entry/exit paths, actions, keyboard/focus behavior, normal data, empty,
   loading, partial, stale, disabled, unsupported, validation, permission,
   cancellation, disconnected, reconnecting, error, destructive-confirmation,
   narrow-layout, large-data, and recovery states.
4. Map every manifest item and state to its owning plan checkpoint, Rust
   contract, CLI/TUI implementation, native implementation, automated tests,
   and evidence artifact. Missing mappings require new or amended plan steps;
   never leave them as informal follow-ups.
5. Review plan dependencies and acceptance gates after every completed plan and
   whenever implementation reveals missing behavior. Split or extend plans as
   needed, assign exact verification commands, add them to `plans/README.md`,
   and continue in dependency order.
6. Render and exercise every surface in both clients with representative
   PostgreSQL, ClickHouse, and Redis fixtures where applicable. Inspect actual
   visuals and interaction, not only model state or source code.
7. Repeat gap analysis against product docs, TablePro's allowed public workflow
   evidence, platform conventions, runtime behavior, and test coverage until a
   fresh pass finds no missing screen, state, interaction, or evidence.

Prompt or plan text may be revised when evidence exposes ambiguity, missing
scope, stale platform assumptions, or inadequate gates. Revisions must expand
or clarify the route to the approved outcome, never redefine completion around
what is already easy or implemented.

Never substitute a smaller compatible implementation, mock-only proof, visual
placeholder, or documentation claim for required production behavior. Do not
defer required work as polish. Do not add excluded product scope.

## Checkpoint loop

For each smallest buildable checkpoint:

1. Read its full plan, authorities, live implementation, tests, and latest
   primary documentation. Run the plan's drift and prerequisite gates.
2. Identify why the architecture permits each observed bug class. Prefer a
   structural fix that removes the enabling condition; use a symptom patch
   only when the root fix is proven infeasible and record the deferred cause.
3. Define failure, cancellation, bounds, safety, redaction, accessibility,
   migration, and recovery behavior before dependent UI work.
4. Implement the complete vertical behavior through shared Rust contracts and
   each applicable client. Remove superseded approaches; carry one architecture.
5. Add unit, integration, process/PTY, rendering, real-server, Swift bridge,
   AppKit/SwiftUI, accessibility, performance, packaging, and clean-machine
   tests proportional to the changed surface.
6. Run every exact plan command plus all applicable repository CI commands.
   Inspect outputs and artifacts; a command exiting zero proves only what that
   command actually covers.
7. Update product/architecture docs, ledger, plan status, evidence index,
   support matrix, provenance, and user documentation with the behavior.
8. Review the full diff for scope, secrets, copied expression, stale paths,
   bypasses, and unrelated changes.
9. Commit on `main` using Conventional Commits, `git commit -s`, and
   `Co-authored-by: Codex <codex@openai.com>`. Push immediately, verify remote
   `main`, verify required CI, then continue without waiting.

Red CI is work to diagnose and fix, not a reason to abandon the program. A
flaky test must be structurally repaired or proven external; rerunning until
green is not completion evidence.

## UI and coverage gates

UI completion requires observable, state-complete proof:

- CLI/TUI: deterministic render-harness coverage at required terminal sizes;
  keyboard, mouse, focus, paste, resize, PTY lifecycle, color-independent
  status, empty/loading/error/unsupported states, dialogs, and every actionable
  control exercised through production update/effect paths.
- Native macOS: XCTest coverage for presentation and bridge state; XCUITest or
  equivalent real-app automation for windows, menus, commands, toolbars,
  sheets, popovers, tables, outlines, editors, settings, file panels, focus,
  keyboard, IME, VoiceOver identifiers/actions, restoration, and multi-window
  ownership. Tests drive shipped app surfaces, not parallel test-only UI.
- Visual evidence: deterministic TableRock-owned screenshots or render
  snapshots for every canonical screen and material state at defined sizes,
  light/dark appearance, Increase Contrast, reduced transparency/motion where
  applicable, and narrow/minimum-size layouts. Never use TablePro screenshots
  as assets or golden images.
- Data/engine matrix: representative small, empty, wide, long, binary, NULL,
  Unicode, structured, temporal, paged, truncated, stale, permission-denied,
  disconnected, and large-result fixtures across each applicable engine.
- Coverage audit: generate current Rust and Swift coverage reports, inspect
  uncovered production paths, and add behavioral tests for every reachable
  safety transition, error/cancel race, screen state, command, and interaction.
  A percentage alone is insufficient; no required manifest row may lack direct
  automated coverage. Exclusions must be unreachable/platform-generated code
  and documented by category with evidence.
- Traceability: maintain one machine-checkable matrix linking requirement ID,
  screen/state/action, client, engine, implementation, test, evidence, and
  status. CI rejects missing links, duplicate IDs, stale paths, or `DONE` rows
  without passing evidence.

## Required completion evidence

Completion requires direct current evidence for all of these:

- Every step, command, artifact, checkbox, test, evidence requirement, and done
  criterion in every numbered file under `plans/` passes; every plan row is
  `DONE` except a genuinely inapplicable plan explicitly marked `REJECTED` with
  approved evidence.
- Every Phase 0–15 exit criterion passes and roadmap language contains no
  partial, mostly-complete, residual, deferred, blocked, or unverified in-scope
  work.
- Every in-scope functional-parity-ledger row is implemented and verified in
  both clients where applicable; unsupported behavior is explicit and only for
  truly inapplicable engine capabilities.
- Canonical screen manifest contains every interface and required state; each
  row has production implementation, direct automated coverage, inspected
  render/runtime evidence, and a passing traceability check for both clients
  and every applicable engine.
- PostgreSQL, ClickHouse, and Redis real-server matrices pass through shared
  Rust contracts, CLI/TUI workflows, and native workflows.
- CLI/TUI terminal lifecycle, TEA purity, bounded streaming, editing safety,
  cancellation truth, files, history, data movement, administration,
  accessibility/non-color cues, and failure recovery pass.
- Native app uses the selected latest stable macOS/Xcode/Swift stack and passes
  strict concurrency, UniFFI ownership, AppKit/SwiftUI interaction, IME,
  VoiceOver, keyboard, appearance, multi-window, restoration, crash recovery,
  bounded memory, performance, and real launch tests.
- Developer ID signing, hardened runtime, notarization, stapling, upgrade,
  clean-machine install, launch, uninstall, and residue checks pass when
  credentials are available.
- Plan 022 preview workflow passes both dispatch and organic green-main
  triggers; artifacts and attestations match source; Homebrew formula and cask
  install, launch/version, uninstall, and pull-verification gates pass.
- All required evidence documents are indexed and reproduce the claims. Docs,
  support matrix, migrations, licenses, and published artifacts match code.
- TableRock, any modified TermRock repository, and the Homebrew tap are clean
  on synchronized `main`; no unpushed commit, untracked deliverable, stale
  release, or mismatched source SHA remains.

Before declaring completion, perform a fresh requirement-by-requirement audit
against current files, test output, CI, runtime behavior, artifacts, and remote
state. Then perform one final TablePro-informed clean-room workflow gap review
and one full screen-manifest replay. Missing, indirect, stale, or uncertain
evidence means not complete. Completion requires two consecutive full audits
with no newly discovered requirement, screen, state, test gap, stale document,
or failed gate; record both audits independently.

## STOP conditions

Stop only when continued progress requires an operator-only credential,
irreversible external action, unavailable repository authority, or a proven
tool/platform limit after safe alternatives are exhausted. Honor a plan STOP
only when its underlying condition still exists in current state; ordinary
bugs, failing tests, stale assumptions, missing implementation, large scope,
time, context limits, and cross-session work are not STOP conditions.

When stopped, leave all completed work committed, pushed, tested, documented,
and clean. Report the exact evidence, affected requirement, last completed
checkpoint, repository/CI state, alternatives attempted, and one concrete
operator action needed to resume. Never mark the plan or full goal `DONE` while
any required item remains blocked.

## Exclusions

Do not add databases beyond PostgreSQL, ClickHouse, and Redis; cloud identity
or proxy products; AI/MCP features; marketplaces; iOS/iPadOS; commerce;
daemon/RPC architecture; WebView UI; manual C ABI; Mac App Store delivery; or
competing parser, persistence, bridge, or TUI stacks. These exclusions do not
remove any requirement already present in plans 001–022 or the in-scope parity
ledger.
