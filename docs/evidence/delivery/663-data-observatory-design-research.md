# Design research: TableRock as Data Observatory

Date: 2026-08-09

## Clean-room

This note records **workflow existence** and design *principles* informed by
public discussion of native DB clients, award-class Mac apps, and Apple HIG /
Liquid Glass. It does **not** copy TablePro/TablePlus/Jackin layouts, colors,
key bindings, assets, product strings, or source.

Competitors establish that:

- native Mac clients can feel direct and light (TablePlus-class);
- platform quality and staged edits matter (TablePro-class *workflow*);
- relationship exploration often lives in separate schema tools (Azimutt-class
  *existence*);
- award apps reward one coherent idea + progressive disclosure + accessibility
  built into the core path.

TableRock’s product idea is **original**:

> **Data Observatory — reveal context and consequence before the operator
> changes anything.**

## Three signature surfaces

| Signature | Operator meaning | TableRock mapping (shipped / next) |
|---|---|---|
| **Relation Lens** | See how selection relates to the rest of the DB | Catalog + Follow FK / structure; UI label **Lens**; progressive neighborhood |
| **Change Ledger** | Every edit is a staged, reviewable, reversible proposal | Existing staged drafts + review dialog; status **ledger N**; Apply is explicit |
| **Environment Halo** | Dev / staging / production are unmistakable without color alone | Profile environment tags + safety; context bar **HALO …** text; production badge |

## Market whitespace

Table-first clients are fast but weak on relationships and consequence.
IDE-first tools are deep but dense. Schema-first tools visualize relationships
but leave daily query/edit. TableRock targets **context-and-consequence-first**
while keeping TablePlus-like directness and Tahoe Liquid Glass chrome.

## Validation targets (checkable)

* Identify environment from status/halo text in &lt;1s (non-color).
* Reach related data via Lens in ≤2 intentional actions when FK facts exist.
* Every write appears in ledger/status before apply.
* Sample path: connect sample → inspect relation → stage → review → revert.
* Core path keyboard-reachable; production never color-only.

## Non-goals (this checkpoint)

* Full ER canvas as primary mode.
* AI agent as product identity.
* Award submission claims.
* Copying competitor chrome.

## Evidence of implementation (this delivery)

* Product copy + TUI/native hierarchy for Halo / Ledger / Lens.
* Tests: context-bar halo wording, ledger status suffix, Lens action identity.
