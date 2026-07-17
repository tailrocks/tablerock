# TermRock migration 0022: paste payloads

Date: 2026-07-17

TableRock advances its exact TermRock `main` pin from `a9774f5` to `d7c998a` and
applies sequential migration `0022-v0.11.0-paste-payload.md`.

## Before

`termrock::input::Event::Paste` was a unit variant and `Event` was `Copy`.
TableRock therefore intercepted Crossterm paste directly to avoid losing the
payload, while its neutral-event path discarded paste.

## After

`Event::Paste(String)` owns the backend payload and `Event` is clone-only.
TableRock removes its competing Crossterm paste path: every backend event first
converts to TermRock's neutral vocabulary, then `InputAdapter::map_event`
converts the owned paste text into bounded root `Message::Paste` intent. No
payload-free variant or compatibility adapter remains.

The same TermRock commit adds grapheme-safe single-line
`TextInputState::insert_str`, exhaustive semantic-role guards, pinned preset
tests, and unknown-future-Crossterm fallback. TableRock does not yet own a
single-line product form at this checkpoint, so no additional widget migration
is required.

Evidence: neutral and backend paste mapping tests, full workspace tests/lint/
rustdoc, exact remote diff, refreshed lockfile, and unchanged redaction/bounds.
No external-product source or protected expression influenced this migration.
