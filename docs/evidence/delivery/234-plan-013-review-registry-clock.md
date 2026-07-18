# Plan 013 — Consume-once review registry across UI clock

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `EffectExecutor.mutation_reviews: MutationReviewRegistry` | done |
| `Effect::ReviewMutations` builds typed plan, `review`, registry `insert` | done |
| `Effect::ApplyMutations` is handle-only (`review_token_hex`) | done |
| Wall-clock issue/expiry (30s within 60s plan max) | done |
| Expired/missing/scope fail → `needs_re_review: true`, never bypass | done |
| TUI: Apply without token blocked; ready stores token; re-review clears | done |
| Unit test: review → apply handle / re-review clear | done |
| Kind-aware cell stage gate (bool/number) | done (partial editors) |

## Policy (STOP satisfied)

Token expiry does **not** weaken authorize. Failed authorize consumes the
token (core registry contract). UI surfaces re-review; operator must Review
again. Preview lines remain descriptive only.

## Verification

```text
cargo test -p tablerock-tui --lib apply_requires_review
cargo test -p tablerock-tui --lib
cargo test -p tablerock-cli --lib
cargo test -p tablerock-core --test mutation
```
