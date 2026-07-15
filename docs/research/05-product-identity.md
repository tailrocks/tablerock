# Product Identity

## Working decision

Use **TableRock** for the product and `tablerock` for the CLI/package prefix:

```text
Repository: tailrocks/tablerock
CLI:        tablerock
Crates:     tablerock-core, tablerock-engine, tablerock-tui, tablerock-cli
Future app: TableRock for macOS
```

The name connects the primary grid/table experience with Tailrocks while
remaining short enough for commands and application chrome. “Table” does not
mean relational-only: the product is positioned as a multi-model data
workbench, and Redis receives a native key/value experience.

## Preliminary search

Checks on 2026-07-15 found no prominent database client or crates.io package
with the exact TableRock name. GitHub has unrelated repositories, the phrase is
widely geographic, and active businesses use “Table Rock,” including technology
services. An old U.S. registration found by the preliminary search is cancelled.

This is not trademark or international legal clearance. Before a product
release, check relevant trademark classes/markets, App Stores, domains,
Homebrew/package managers, social names, and confusing software uses with
appropriate legal review.

## Alternatives considered

- **Lode:** engine-neutral and fits the Tailrocks geology theme, but package and
  repository usage is crowded.
- **Strata:** expresses multiple data models, but is heavily used in software.
- **Quarry:** suggests exploration and extraction, but is extremely crowded.
- **Outcrop:** good visibility metaphor, but existing technical/geoscience uses
  are substantial.
- **Datalith:** strong data/rock meaning, but an existing data-visualization
  component project and software company use it.

TableRock is the best working identity despite the legal-clearance caveat.
