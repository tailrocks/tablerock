# TermRock 0.9 Unified Key Vocabulary Migration

Status: accepted and integrated on 2026-07-16.

TableRock advances its exact TermRock `main` pin from `2f441cc` to
`7d8569dcd549e04ac17a5633196f602b52a134c6` (`0.9.0`). Historical integration
documents remain unchanged evidence of their actual pins.

The public API migration lands at ancestor `561e2dd`. The selected descendant
adds only Form and DetailTable interaction characterization tests; it changes no
public API and therefore requires no additional migration file.

## Sequential upstream migration

TermRock `MIGRATING.md` links the separate
`0006-v0.9.0-unified-key-vocabulary.md` before/after document. The new API
removes `keymap::LogicalKey` and `keymap::Mods`; `KeyChord` now directly owns
`input::KeyCode` and `input::KeyModifiers`. Raw modifier bit layouts changed,
so persisted numeric bits require value migration rather than reinterpretation.

## TableRock adaptation

TableRock does not consume TermRock keymap types or persist TermRock modifier
bits. Its CLI adapter currently maps Crossterm events into TableRock root TEA
messages. Therefore the correct migration is an exact dependency repin with no
compatibility aliases, conversion layer, or dormant old vocabulary.

Any future reusable binding surface must use `termrock::input::KeyCode`,
`KeyModifiers`, and named modifier constructors directly. It must never persist
their implementation bit pattern as a stable TableRock wire format.

## Verification

- Workspace compilation and tests exercise the new exact revision.
- Search confirms TableRock owns no `LogicalKey`, `Mods`, or TermRock
  `KeyChord` use.

External concepts: one backend-neutral input vocabulary
Public source: <https://github.com/tailrocks/termrock/tree/7d8569dcd549e04ac17a5633196f602b52a134c6>
Implementation source: exact pin adaptation only
Copied code/assets/text: none
