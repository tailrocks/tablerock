# sqlparser adoption + statement boundaries

Date: 2026-07-18

## Checkpoint

Plan 011 step 1. Adopt `sqlparser` `=0.62.0` (Apache-2.0, latest stable at
adoption) for dialect-aware statement spans. Module lives in
`tablerock-core` (pure; shared by TUI + engine). Engine re-exports. Never
naive `split(';')`.

## Dependency adoption

| Field | Value |
|---|---|
| Crate | `sqlparser` |
| Version | `=0.62.0` |
| License | Apache-2.0 |
| Features | `std` only (default features off) |
| MSRV | workspace `1.97` |
| Motivation | Fixed decision "SQL/editor path": tokens + last-known-valid AST; dialect-aware boundaries with incomplete-input fallback |
| Alternatives rejected | Hand-rolled lexer (higher maintenance); naive semicolon split (wrong on strings/dollar-quotes) |

## API

- `SqlDialect::{PostgreSql, ClickHouse}`
- `statements(source, dialect) -> Vec<StatementSpan>`
- `statement_at(source, dialect, cursor) -> Option<StatementSpan>`
- Spans are UTF-8 byte offsets; `complete` false for open/incomplete tails.

## Recovery

1. Tokenize with location via sqlparser dialect.
2. Split on top-level `SemiColon` (paren/bracket/brace depth).
3. On tokenizer error (unterminated string, etc.): salvage complete prefixes,
   then character-level recovery for the remainder (quotes, `$$` tags,
   line/block comments).

## Evidence

- `cargo test -p tablerock-core --lib sql_analysis` (7 tests):
  string-embedded `;`, dollar-quoting, incomplete string, comments/emoji,
  `statement_at`, empty, `E''` + line comments.

## Provenance

Primary: Apache DataFusion sqlparser-rs docs + fixed-decisions SQL/editor path.
No external product source consulted for algorithm.
