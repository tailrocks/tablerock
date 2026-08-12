# Change Review craft pass

Date: 2026-08-09

> Superseded in part by delivery evidence 727. The hard-coded probe workflow
> described below was removed from production; real DDL, table-operation, CSV,
> and role-change review paths remain.

## Path

```text
Rust MutationReviewRegistry / stage_* tokens (UniFFI)
  → Workbench*Review models (tokenId, preview, destructive, expiresAtMs)
  → ChangeReviewPresentation (pure facts: kind, DESTRUCTIVE, expiry, ledger chip)
  → ChangeReviewPlane + review sheets (SwiftUI, opaque content)
  → Apply / Discard → apply* / revoke* (Rust still owns authority)
```

Probe path:

```text
Review probe… → stageProbeReview (token only)
  → ProbeChangeReviewSheet (documented DELETE preview)
  → Apply Reviewed Change → applyReviewToken
  → Discard → revokeReviewToken
```

## Research (principles only)

| Reference class | Principle |
|---|---|
| Git clients (commit / staging) | Consequence visible before write; discard is first-class |
| Xcode / pro review sheets | Kind-first chrome; monospaced plan; opaque content |
| Own Continuum / Value Inspector craft | Uppercase section labels; dense metadata strip |
| HIG confirmation | Destructive role + non-color DESTRUCTIVE word |
| Product `editing.md` Change Ledger | Ledger count in status; review shows parameterized plan |

Competitors establish that review-before-apply exists. No layouts or copy copied.

## Before

| Area | Problem |
|---|---|
| Apply probe | Silent stage+apply — no review surface |
| Review sheets | Inconsistent GroupBox forms; orange-only danger |
| Hierarchy | Generic titles; no shared instrument language |
| Ledger | Status bar never showed pending review count |
| Production | Halo elsewhere; not on frozen plan plane |

Excellent kept: consume-once tokens, exact confirmation for table ops,
DDL second confirmation, CSV freeze, opaque plans never reparsed for apply.

## Design decision

One coherent direction: **Change Review instrument**.

| State | Behavior |
|---|---|
| Resting | No LEDGER chip; probe button idle |
| Review open | Shared plane: KIND · LEDGER · monospaced preview · metadata · Apply/Discard |
| Destructive | Word **DESTRUCTIVE** (not color alone) + existing confirmations |
| Production | HALO PRODUCTION on plane when profile production |
| Loading | ProgressView applying… |
| Empty (probe sheet after apply) | Outcome label; no fake open token |
| Error | Monospaced error; token cleared after failed apply (consume-once honesty) |
| Probe | Stage only → sheet; documented probe DELETE shape (Rust contract) |

Fixed: token/apply ownership in Rust. Moves: sheet presentation only.
Permanent: kind + preview + authority facts. Progressive: secondary sheets.

Why better: consequence before mutation is visible; hierarchy matches Data
Observatory craft; experts see ledger presence without hunting sheets.

## Implementation

| File | Change |
|---|---|
| `WorkbenchTypes.swift` | `ChangeReviewPresentation`; status facts ledger params; `stageProbeReview` |
| `TableRockApp.swift` | `ChangeReviewPlane`, `ProbeChangeReviewSheet`; DDL/table/CSV planes; status LEDGER; stage/apply/discard probe |
| `ChangeReviewPresentationTests.swift` | Pure tests |
| `editing.md` / design-system / parity | Product truth |

## Validation

```text
cd native && swift build --target TableRockFeature
cd native && swift build --target TableRockApp
```

Both passed (macOS Command Line Tools host). Full `swift test` needs Xcode for
XCTest suites; pure Testing cases added for CI.

Preserved ids: `structure.change.preview`, `structure.change.apply-review`,
`table-operation.preview`, `table-operation.apply`, `import.csv.sheet`.

New ids: `change.review.plane`, `change.review.probe.sheet`,
`query.review-probe`, `workbench.status.ledger`.

## Remaining weaknesses

* Full cell-edit MutationDraft → native review list still TUI-ahead.
* Role change still uses confirmationDialog rather than full plane.
* Expiry countdown does not tick live (snapshot at render).
* No interactive GUI light/dark capture on CLT agent.

## Next improvement for Change Review

**Native staged cell draft inventory** (per-tab ledger list + discard one) from
Rust draft projection — same plane language, real grid edits not only probe.
