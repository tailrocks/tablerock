# SQL Editor

SQL tabs pair a multiline editor with results. They exist for PostgreSQL and
ClickHouse; Redis gets a command editor with the same shape (see
[Redis screens](redis.md)).

## Layout

```text
+-------------------------------------------------------------+
| editor: multiline SQL, line numbers, current statement mark |
+-------------------------------------------------------------+
| run · cancel · format · history          elapsed · rows     |
+-------------------------------------------------------------+
| results: grid per statement, or error with position         |
+-------------------------------------------------------------+
```

Editor above results by default; the split is resizable and remembered.

## Editing

- Unicode-safe multiline buffer: cursor/selection, undo/redo, search,
  paste, line numbers, horizontal scroll.
- Syntax spans and diagnostics are computed outside rendering; incomplete
  text never breaks highlighting or crashes the parser.
- Statement boundaries come from a dialect-aware parser — never naive
  semicolon splitting — so procedures, comments, and strings behave.
- The current statement is visibly marked; **Run** executes the selection
  when one exists, else the current statement.

Find and Replace is explicit editor authority. Both clients expose previous,
next, replace, and bounded replace-all actions over either the whole document
or a frozen current-selection scope. Modes are case-insensitive literal,
case-sensitive literal, Unicode word, and regular expression. Invalid patterns
and empty selection scopes remain visible errors. Zero-width regular-expression
matches advance without looping; replace-all stops above 10,000 matches.

## Autocomplete

Typing opens a completion popup anchored at the cursor:

- candidates: keywords, tables, views, columns, functions, types from the
  live catalog of the tab's database/schema context, plus aliases parsed
  from the query text;
- schema-aware: after `schema.` it lists that schema's objects; after
  `table.` it lists columns;
- keyboard and mouse both navigate; Enter/Tab commits, Escape dismisses;
- results are revisioned: candidates computed against older text or an older
  catalog never apply;
- the popup never covers the cursor and flips/clamps inside the editor.

## Execution and results

- Run shows queued, running, streaming, completed, cancel requested,
  cancelled, failed — with elapsed time and loaded rows/bytes. Cancel is
  always reachable while running and reports the observed outcome.
- Multiple statements produce one result section per statement, in order,
  each with its own summary (rows, timing, command tag) and failure state;
  one failure does not hide earlier results.
- Results render in the same grid as table browsing: sorting, copy formats,
  inspector included. Arbitrary query results are read-only; editability
  follows the rules in [Editing and review](editing.md).
- Engine errors show redacted message plus severity/position where the
  engine provides it, mapped back into the editor.
- Explain runs the active SQL through a Rust-owned engine intent. PostgreSQL
  uses text-plan `EXPLAIN` without `ANALYZE`; ClickHouse uses its plain
  `EXPLAIN`; Redis is explicitly unsupported. Returned plan lines open in a
  selectable, copyable native viewer and remain available as ordinary result
  data. Already-explained SQL is never double-prefixed.

## History

- Executed statements enter a bounded, searchable local history with
  configurable SQL-text retention and a disabled/private mode.
- History entries restore into the current tab's editor; they never
  auto-execute.

## Saved queries and files

Named saved queries and `.sql` files arrive with the workbench-foundation
phase: open, save, atomic writes, external-change detection, and the same
unsaved-change policy as staged edits.

## Both clients

| | TUI | Native macOS |
|---|---|---|
| Editor | TermRock `TextArea` behind a TableRock model | `NSTextView`/TextKit with IME and native find |
| Completion | TermRock `CompletionMenu` | native completion presentation |
| Results | `VirtualGrid` | `NSTableView` |
| Diagnostics | inline marks + status | inline marks + inspector |

Completion candidates, statement boundaries, execution, and result truth are
Rust-owned in both clients; the native editor never parses SQL for behavior.
