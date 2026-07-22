# Remove Vim mode

Date: 2026-07-22

The optional Vim keymap is outside TableRock's product scope. This checkpoint
removes the TUI keymap module, reverts the unpublished native surface from the
preceding checkpoint, and removes the capability from active product,
architecture, roadmap, and screen-manifest contracts.

Published history remains forward-only. No shared editor primitive was removed;
the neutral TermRock-backed editor remains the sole editing model.

## Verification

```text
rg -n 'Vim|vim_mode|vim mode|vim-mode' crates native docs/product docs/architecture scripts
cargo test -p tablerock-tui
swift build --package-path native -c release
cargo test -p tablerock-core --test screen_manifest
```

The search returns no active product, architecture, implementation, or script
references. Roadmap and delivery evidence retain only the retirement record.
