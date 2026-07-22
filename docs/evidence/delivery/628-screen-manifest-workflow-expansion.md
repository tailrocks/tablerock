# Evidence 628: screen-manifest workflow expansion

Date: 2026-07-22

## Claim

Ten required, independently operable surfaces formerly collapsed into broad
manifest rows now have stable IDs and per-client status:

- SQL find/replace;
- typed query parameters;
- PostgreSQL structure;
- table-operation authority;
- import progress/outcome;
- export progress/outcome;
- SSH configuration;
- startup-command editor;
- Vim-mode state;
- PostgreSQL maintenance outcome.

PostgreSQL structure is `partial` in both clients. Remaining rows are TUI
`partial`, native `missing`. No missing native surface points to a placeholder
implementation or test.

## Source and clean-room provenance

Rows derive from TableRock's product documents, functional parity ledger, and
plans 013, 016–018. Current public TablePro overview, connection, and
import/export documentation was used only to check broad workflow classes.
No external source, tests, text, identifiers, assets, screenshots, geometry,
colors, or key bindings were copied.

## Verification

```text
mise exec -- rtk cargo test -p tablerock-core --test screen_manifest
cargo test: 1 passed (1 suite, 0.00s)

mise exec -- rtk cargo clippy -p tablerock-core --test screen_manifest -- -D warnings
cargo clippy: No issues found

rtk git diff --check
exit 0
```

## Residual

All new native gaps block parity. `partial` TUI status also remains unproven
until direct state/action replay and inspected render evidence replace broad
suite links.
