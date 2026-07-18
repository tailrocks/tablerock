# Typed browse-plan builder

Date: 2026-07-18

## Checkpoint

Plan 012 step 1. `crates/tablerock-engine/src/browse_plan.rs` owns
`BrowsePlan` → parameterized SQL. Identifiers via `quote_ident` /
`qualify_table`. Filter values are `$n` parameters only. Raw WHERE is
parenthesized AND-composed; any `$n` token inside raw WHERE is rejected
(fail closed, no renumbering).

## Decision

- Raw WHERE `$n` collision policy: **reject** (document in STOP resolution).
- `FilterValue` Debug redacts text/integer/float payloads.
- LIMIT/OFFSET are plan integers, not user strings.

## Evidence

- `cargo test -p tablerock-engine --lib browse_plan` (12 tests):
  hostile table name quoting, sort keys, typed filters, IS NULL, missing/
  unexpected values, raw WHERE compose, `$1` collision, empty raw, Debug
  redaction, empty ident.

## Remaining (plan 012)

- Sort UI + re-run via plan builder
- Filter bar + page-local quick filter
- Column layout persistence
- Copy formats + clipboard effect
