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

The implementation plan uses only TableRock. The legal-clearance gate may block
public release, but no second working name appears in product code or roadmap.
