# Plan 019 checkpoint — real PostgreSQL apply-by-handle

Date: 2026-07-18

## What landed

- `insert_reviewed_probe` accepts optional relation + locator id (Rust-only;
  still never exposes plan bytes over UniFFI).
- Real-server test `bridge_postgres_apply_delete_by_review_token`:
  1. Open PostgreSQL via facade
  2. `execute` create/clear/insert into `bridge_apply_probe`
  3. Insert reviewed delete handle for `id = 7`
  4. `apply_review_token` commits apply
  5. Second apply with same handle rejects (`authorize` — consume-once)

```bash
cargo test -p tablerock-ffi --test bridge_real bridge_postgres_apply
# 1 passed
cargo test -p tablerock-ffi --test bridge_real
# 5 passed
```

## Contract

shared-client-contract.md: handle-based apply; no plan bytes on the bridge;
failed or successful apply cannot reuse the same review token.
