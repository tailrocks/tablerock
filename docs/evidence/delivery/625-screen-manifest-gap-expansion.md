# Evidence 625: screen-manifest gap expansion

Date: 2026-07-22

## Claim

The canonical screen manifest now records ten previously omitted required
surfaces as explicit `missing` work. The verifier permits an applicable client
implementation or test link to be `n/a` only while the entire row is honestly
`missing`, and rejects a missing row that identifies no missing implementation
or test. This removes the earlier structural pressure to point absent native
work at a broad source or test file.

Added surfaces:

- connection URL import;
- external URL-open confirmation;
- quick switcher;
- explain-plan viewer;
- PostgreSQL activity;
- PostgreSQL backup/restore;
- relationship browser;
- PostgreSQL roles/privileges;
- Redis Pub/Sub console;
- structure-change review.

These rows do not claim behavior complete. Each names its current TUI seam,
the absent native seam, direct earlier evidence, and a concrete replay gap.

## Root cause

The foundation verifier required non-`n/a` TUI and native paths for every
`both` row regardless of `status`. That rule made omitted surfaces easy to
exclude and missing-client work impossible to represent without a misleading
broad-file link. Status-aware validation removes that enabling condition.

## Clean-room workflow review

Current public TablePro documentation was reviewed only to identify broad
database-workbench workflow classes:

- <https://docs.tablepro.app/features/overview>
- <https://docs.tablepro.app/features/import-export>
- <https://docs.tablepro.app/databases/overview>

The review corroborated workflow classes already required by TableRock's
parity ledger: URL handling, explain, activity/dashboard, relationship/schema
inspection, reviewed structure editing, and progress/cancellation for data
movement. No TablePro source, tests, identifiers, assets, product text,
screenshots, layout measurements, colors, or key bindings were read or copied.
TableRock names, state profiles, ownership links, and acceptance gaps derive
from TableRock requirements and implementation.

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

All new rows remain `missing`. Existing rows remain `partial`. Next closure
checkpoint must add per-client direct implementations/tests and replay every
required state; this inventory expansion is not parity evidence.
