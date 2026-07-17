# TermRock 0.10 Runtime Keymap Spike Update

## Upstream change

TableRock now pins exact TermRock `main` revision
`bbc6c980389e49f4306a8e65a71ce11f280147e7`. Relative to the prior pin
`ade6aa0b394b0afac1da0237d83390b9d5441668`, this revision adds a
`#[cfg(test)]` copy-on-write runtime keymap prototype and its design record. It
does not change the compiled public crate API and adds no migration file.

The accepted direction keeps one keymap representation: static borrowed
defaults become owned only on the first runtime edit, while dispatch, hints,
glyphs, and conflict checks read the same resolved bindings. This matches
TableRock's forward requirement for eventual user-remappable shortcuts without
parallel legacy/default lookup paths.

## TableRock impact

No source migration is possible or required yet because the prototype is test
only. TableRock retained its then-current static routing. TermRock later
published migration 0025, adopted without a compatibility override layer in
[`132-termrock-runtime-keymap-migration.md`](132-termrock-runtime-keymap-migration.md).

## Verification

- Exact Git revision resolves in the lockfile.
- Workspace tests, lint, and documentation pass against the revision.
- No TableRock behavior or support claim changes.

External concepts: copy-on-write keymap storage
Public sources: <https://github.com/tailrocks/termrock/commit/bbc6c980389e49f4306a8e65a71ce11f280147e7>
Implementation source: TermRock upstream test-only prototype and TableRock-owned compatibility inspection
Copied code/assets/text: none
