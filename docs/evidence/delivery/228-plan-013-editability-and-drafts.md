# Plan 013 — Editability facts + staged draft model

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `EditabilityFacts` / `EditabilityReason` / `StableIdentity` (core) | done |
| Profile ReadOnly, non-base, no table, no key → read-only with reason | done |
| Cell gate: truncated / invalid / unknown not writable | done |
| `MutationDraftModel` per tab (insert/update/delete, undo, discard) | done |
| Markers: inserted / modified / deleted; original value reachable | done |
| ReadOnly: staging blocked; drafts discarded on policy demotion | done |
| Grid status: staged count + read-only reason | done |
| Typed cell editors / review dialog / apply UI | next |
| Catalog PK facts → identity_columns wiring | next |

## Verification

```text
cargo test -p tablerock-core --lib editability
cargo test -p tablerock-tui --lib mutation_draft
cargo test -p tablerock-tui --lib
```

## Provenance

Built from `docs/product/editing.md` editability + staging rules and core
`ProfileSafetyMode`. No third-party product source.
