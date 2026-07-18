# Plan 019 checkpoint — apply-by-handle + disconnect

Date: 2026-07-18

## What landed

UniFFI facade additions:

- `apply_review_token(token_id, now_ms, session_id, expected_revision) -> ApplyOutcome`
  - Consumes the review-token handle **before** driver apply
  - Failed apply cannot be retried with the same handle (ambiguous-write non-retry)
  - Outcome is a safe summary only (transaction label + counts; no SQL/cells)
- `disconnect(session_id)` — engine disconnect once idle; drops bridge session map entry

Conformance:

- `apply_review_token_consumes_handle_even_when_apply_fails`
- `disconnect_rejects_unknown_session`

```text
cargo test -p tablerock-ffi --test facade --test conformance
# 15 passed
```

## Contract alignment

shared-client-contract.md: mutation registry is handle-based; serialized plan
bytes never cross the bridge. Swift may later render previews only after Rust
issues a token id.

## Residual operator STOP (unchanged)

XCFramework full packaging + Developer ID notarization still require operator
Xcode/certs (evidence 251).
