# Clean-Room Reference Policy

The reference boundary is deliberately stricter than the minimum license
question. TableRock learns which problems exist without inheriting another
product's implementation or distinctive expression.

## TablePro

The [TablePro repository](https://github.com/TableProApp/TablePro) currently
uses the [GNU AGPL-3.0 license](https://github.com/TableProApp/TablePro/blob/main/LICENSE).
Its public documentation and screen structure may establish connection,
catalog, tab, grid, editor, result, and edit workflows as market expectations
(operator revision 2026-07-18). Its source is never an implementation source
for Apache-2.0 TableRock.

Do not copy or translate TablePro source, tests, comments, identifiers, assets,
text, layout measurements, or plugin architecture. Screen *structure* (a
sidebar catalog beside tabbed content, a filter bar above a grid) is a common
workflow pattern; screen *expression* (geometry, strings, icons, colors,
shortcuts) is copied from nothing and derived only from TableRock's product
specification. TablePro solves a broad many-driver Apple-client problem;
TableRock deliberately starts with three built-in Rust adapters.

## TablePlus

TablePlus is proprietary. Evidence is limited to its
[public documentation](https://docs.tableplus.com/) and commonplace market
expectations. Do not inspect or reverse engineer its implementation or derive
detailed designs from it.

## Zedis

[Zedis](https://github.com/vicanso/zedis) uses
[Apache-2.0](https://github.com/vicanso/zedis/blob/main/LICENSE), but the
operator requires concepts-only use. Its public feature documentation may
establish Redis needs such as incremental discovery, type views, TTL context,
commands, current status, and production-aware safety. Do not copy its source,
tests, assets, text, geometry, colors, or key bindings.

## Allowed evidence

- public user documentation and public feature lists;
- high-level public screenshots only to establish that a workflow exists;
- official PostgreSQL, ClickHouse, Redis, Apple, 1Password, and Ratatui docs;
- selected Rust crate docs and source under their normal dependency review;
- direct experiments and integration fixtures written for TableRock.

## Prohibited use

- copy, translate, mechanically transform, or closely paraphrase reference code;
- port reference state types, protocols, tests, fixtures, UI strings, or layouts;
- reuse screenshots, icons, colors, product text, or key maps;
- accept performance/version claims without independent measurement;
- treat a permissive license as permission to ignore this operator policy.

## Implementation provenance

Every implementation commit influenced by a reference records this block in
its commit body and links the TableRock requirement/test that independently
defines the behavior:

```text
External concept: <broad behavior>
Public source: <documentation URL>
TableRock requirement: <research/issue link>
Implementation source: official protocol/library docs and TableRock tests
Copied code/assets/text: none
```

For close behavior, perform an independent review from the TableRock
requirement and diff without consulting the external implementation. Direct
work on `main` does not weaken this provenance gate.
