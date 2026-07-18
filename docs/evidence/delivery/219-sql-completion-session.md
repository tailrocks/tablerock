# SQL completion session (keywords + catalog)

Date: 2026-07-18

## Checkpoint

Plan 011 step 3 (partial). Pure `CompletionSession` builds keyword + catalog
object candidates from the token under the cursor. Sessions carry three
revision axes (editor text, workbench context, catalog). Stale sessions
refuse commit. Workbench `Complete` opens/commits; `Cancel` dismisses.
View paints TermRock `CompletionMenu` over the editor.

## Decision

- No ranking beyond prefix match (keywords then catalog); parser aliases
  deferred.
- Commit replaces the token byte range only — pure buffer edit (injection
  after open quote still only mutates text).
- Schema-qualified `schema.` / `table.` column drill-down deferred to a
  follow-up when column catalog leaves are always loaded.

## Evidence

- `model::completion::tests::*` — prefix filter, stale text, stale catalog,
  commit replace, quote-prefix injection safety
- `cargo test -p tablerock-tui --lib`
- `cargo test -p tablerock-cli --test pty_lifecycle high_rate`

## Remaining

- Navigate candidates with Up/Down while menu open (key routing)
- Column completion after `table.`
- Alias extraction from query text
