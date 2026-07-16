# `/goal` Prompt: Implement TableRock End To End

## Objective

Implement all of TableRock: a PostgreSQL, ClickHouse, and Redis workbench, first
as a Rust CLI/TUI and then as a native macOS app over the same Rust core. This
invocation approves all fixed decisions and authorizes `ROADMAP.md` Phase 0-15.
Persist across sessions; partial delivery never completes the goal.

## Authority And Scope

Before changes, read `AGENTS.md`, `CONTRIBUTING.md`, `README.md`, `ROADMAP.md`,
and all `docs/research/`; its `README.md` maps them. Completion is `06`; core
architecture is `07`, `10`, `13`, `14`, `20`, and `31`; UIs are `11` and `12`;
execution is `30`; evidence is `32`; delivery is `33`. Conflicts resolve by
`AGENTS.md`, then `31`, then `30`; repair stale text in the same checkpoint.

In scope: the entire three-database ledger, Rust engine, both UIs, reusable
TermRock components, distribution, tests, and docs. Excluded: other databases,
cloud identity/proxies, AI/MCP, marketplaces, iOS/iPadOS, commerce, daemon/RPC,
WebView, manual C ABI, Mac App Store, and competing parser/TUI stacks.

## Fixed Architecture

Rust owns all non-presentation behavior. The TUI uses one root TEA
Model/Message/Update/Effect/Subscription/View flow, TermRock for reusable
primitives, Ratatui rendering, and Crossterm input; I/O stays in effects/engine.
Use `20`/`31` dependencies, including local-only `turso`; never `rusqlite` or
`libsql`. Hide database types behind adapters; pass bounded immutable pages.

macOS embeds Rust through coarse synchronous UniFFI. Swift owns presentation/OS
integration only: SwiftUI structure plus AppKit outline/table/text controls.
Ship with Developer ID, hardened runtime, notarization, and stapling.

Apply `01`: external products prove broad workflows only. Never copy source or
protected expression. Build from requirements, primary docs, and tests; record
provenance.

## Execution Loop

Follow `ROADMAP.md` and `30-delivery-plan.md` dependencies exactly. For each
smallest buildable checkpoint:

1. Inspect code, prerequisites, gates, and primary docs; resolve unknowns by
   inspection, tests, or research.
2. Define failure, cancellation, safety, redaction, and bounds; build core first.
3. Implement/test to `32`; preserve stable adapter boundaries.
4. Update tests, decisions, ledger, roadmap, support matrix, docs, and provenance.
5. Run all applicable workspace/CI gates until green; review drift, copying,
   secrets, and unrelated changes.
6. Commit, push, verify `HEAD == origin/main`, and immediately continue.

If a reusable primitive is missing, add its neutral tested/documented API to
TermRock, push TermRock `main`, pin that revision, integrate, and continue. Jackin
is read-only usage evidence; never import its product internals.

Never retain competing approaches. Make one evidence-backed binary decision for
any unspecified detail and record it before dependent work. Ordinary ambiguity,
failures, context limits, and missing components are not stop conditions.

## Done Criteria

Complete only when every Phase 0-15 exit criterion and applicable ledger row
passes; all three databases work through shared Rust contracts in both UIs; all
required safety, paging, editing, data-transfer, administration, packaging,
notarization, and clean-machine evidence passes; docs match behavior; no bypass
remains; and both repositories are clean and synchronized with remote `main`.

## Stop Conditions

Stop only for a proven external/tool/platform limit or irreversible action
requiring operator authority. Record evidence, failed alternatives, current
checkpoint, clean committed state, and the single action needed to resume. Never
claim completion because work is large, slow, or spans sessions.

## Git

Work only on `main`; never create branches or pull requests. Preserve unrelated
changes. Use atomic Conventional Commits, DCO sign-off, and
`Co-authored-by: Codex <codex@openai.com>`. Push every commit immediately.
