# TermRock T3: TextArea audit + CompletionMenu pin

Date: 2026-07-18

## Checkpoint

Plan 010. TermRock `main` at `dd8bed1` adds `CompletionMenu`. TableRock pins
that revision. TextArea contract audit against delivery-plan T3.

## TextArea contract audit (pin `dd8bed1` / prior `5ab74a2` text_area)

| Contract item | Verdict | Evidence |
|---|---|---|
| Grapheme-safe editing | PASS | `TextCursor` byte offset at extended-grapheme boundary; `edit_core` insert/delete |
| Cursor | PASS | `TextCursor` + `set_cursor` boundary validation |
| Selection | GAP | No range selection API; single cursor only |
| Undo/redo | GAP | No history stack; edits apply immediately |
| Line numbers | GAP | Render paints body lines only; no gutter |
| Search | GAP | No find/next API |
| Vertical + horizontal scroll | PASS | `DialogScroll` + clamp + scrollbar hit geometry |
| Paste (multi-line) | PASS | `insert_text` / `Event::Paste`; CRLF/LF/CR normalize |
| External spans/diagnostics | GAP | No caller span overlay; styles fixed by theme roles |
| Geometry clamping / min rect | PASS | Empty-area safe; scrollbars inside panel tests |
| Jackin-compatible additive APIs | PASS | `CompletionMenu` is additive; TextArea surface unchanged |

## CompletionMenu contract

| Item | Status |
|---|---|
| Stable candidate IDs | PASS — `CompletionCandidate.id` |
| Selected ID | PASS — `CompletionMenuState::selected` |
| Clamp/flip geometry | PASS — `place_completion_menu` tests (below, above, right-edge) |
| Never cover anchor | PASS — intersection guard + flip |
| Scroll | PASS — offset + page/home/end |
| Keyboard | PASS — up/down/page/enter/tab/esc |
| Mouse | PASS — hover select, click commit, outside dismiss |
| Caller ranking/commit | PASS — no ranking/parser in TermRock |
| Lookbook | PASS — `completion-menu/basic`, `completion-menu/edge` |

## Residual (additive follow-ups; do not block plan 011)

TextArea selection, undo/redo, line numbers, search, and external span
overlay remain GAPs. Plan 011 can ship SQL editor policy on current TextArea
and layer overlays locally until TermRock closes those GAPs; prefer upstream
additive extensions when touching edit chrome.

## Pin

- TermRock: `dd8bed132903dbe3a8113d72940f23928716f498`
- Prior: `5ab74a2d03a4bec50ebe5fbc90439ae607e0215d`

## Verification

- termrock: `cargo test -p termrock --lib` (215); lookbook tests; previews check
- tablerock: `cargo test -p tablerock-tui -p tablerock-cli` after pin bump
