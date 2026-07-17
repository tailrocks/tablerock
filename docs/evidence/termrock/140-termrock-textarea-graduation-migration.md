# TermRock TextArea Graduation And Migration 0027

Date: 2026-07-17

## Published change

TableRock now pins exact TermRock `main` revision
`5ff94ee117fd4a1b72fdd0d1b1847815055a93ac`. This revision graduates the public
multiline `TextArea` widget and publishes sequential migration
`0027-v0.11.0-text-input-boundary-repair.md`.

Migration 0027 removes scalar-length cursor advancement after insertion.
`TextInputState::cursor_byte()` is now repaired against global post-edit
grapheme segmentation, including inserted scalars that merge with adjacent
combining or ZWJ content. No public API name changed. Consumers must expect the
first grapheme boundary at or after the logical insertion end, never an
intermediate byte inside an extended grapheme.

## TableRock impact

TableRock does not currently instantiate `TextInput` or `TextArea`, so no
consumer assertion or state migration is required. The root shell builds and
passes its complete TUI/CLI suite at the refreshed pin.

The graduated `TextArea` satisfies the reusable multiline-widget prerequisite
for Phase 5. TableRock will adopt it only after Phase 4's PostgreSQL read-only
vertical slice, supplying Rust-owned document revisions, SQL/Redis syntax,
completion, diagnostics, execution policy, and TEA messages. TableRock will not
carry the removed lookbook-local editor or another textarea stack.

## Verification

- exact remote `main` equals the Cargo pin and lockfile revision;
- TableRock TUI and CLI compile and test with all TermRock features enabled;
- the workspace architecture, real-server, lint, rustdoc, dependency, secret,
  English, and formatting gates pass; and
- no TableRock assertion depends on the removed intermediate cursor behavior.

## Provenance

External concept: grapheme-safe insertion cursor repair and reusable multiline
terminal editing  
Public source: TermRock migration 0027, public API, tests, and documentation at
revision `5ff94ee117fd4a1b72fdd0d1b1847815055a93ac`  
TableRock requirements: research 11, 13, 20, 30, 31, and 32  
Implementation source: TermRock public contract and TableRock compatibility
tests  
Copied code/assets/text: none
