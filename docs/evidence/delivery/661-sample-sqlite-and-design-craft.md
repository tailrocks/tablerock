# Sample SQLite + Liquid Glass craft checkpoint

Date: 2026-08-09

## Checkpoint

- Design research 660 applied: chrome vs content, Try Sample onboarding.
- `Engine::Sqlite` local-file engine (Turso) with sample fixture schema.
- Bridge `prepare_sample_database`, native Try Sample CTA, TUI `TrySample` /
  `OpenSampleDatabase` effect.

## Decision

| Surface | Implementation |
|---|---|
| Sample schema | TableRock-owned artists/albums/tracks/orders (not Chinook) |
| Persist path | Relative `samples/tablerock-sample.db` under operator data root |
| Connect path | Absolute path via host when ≤253 bytes, or resolve relative under data root |
| Native empty state | ContentUnavailable + glass-prominent Try Sample |
| TUI | Action strip Sample → ensure + temporary connect |
| Glass | Toolbar/sidebar/sheets only; grid/editor stay opaque (product doc + greps) |

## Evidence commands

```text
cargo test -p tablerock-engine --lib sample_fixture
cargo test -p tablerock-persistence --test sqlite_profile
cargo test -p tablerock-ffi --test sample_sqlite
cargo test -p tablerock-tui --test craft_hierarchy
rg -n 'glass|GlassEffect|NSVisualEffect' native/Sources
```

## Clean-room

Public workflow existence only (sample onboarding, native simplicity). No
competitor source, layouts, colors, or product strings.

## Verification (2026-08-09, skeptic pass)

| Gate | Result |
|---|---|
| Honest `sample_sqlite` (TABLEROCK_TEST_ROOT, no host rewrite, bridge execute) | pass |
| TUI `ConnectOk` SQLite → `LoadCatalog` Root; expand levels not always Root | pass |
| CLI `catalog_request_for_level` SQLite → SqliteRoot/Tables/Columns | pass |
| Relative sample path resolves under configured persistence parent | pass |
| Native `trySampleDatabase` honors `connect()` failure | pass |
| Sample `password_source=none` survives save→get_profile_draft (not rewritten to prompt) | pass |
| Native connect skips password sheet for sqlite / source none | pass |
| Glass grep (chrome-only); no theater unit tautologies | pass |

## Remaining

- Hosted XCUITest for Try Sample CTA.
- Full SQLite ledger parity beyond sample/read/catalog.
- Subjective “award-winning” claims out of scope (gates above only).
