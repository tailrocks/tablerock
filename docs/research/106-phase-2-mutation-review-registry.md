# Phase 2 Single-Use Mutation Review Registry

## Decision

Copyable client or UniFFI data cannot carry mutation authority. Rust therefore
owns reviewed plans in `MutationReviewRegistry` and exposes only opaque
`ReviewTokenId` values. The registry has a caller-selected nonzero capacity with
a hard product ceiling of 4,096 resident entries.

## Fail-closed lifecycle

- insertion rejects pre-issue clocks, already-expired reviews, duplicates, and
  capacity overflow;
- insertion first purges expired entries so stale authority cannot reserve
  capacity;
- authorization removes the reviewed plan before checking clock, scope, and
  revision;
- successful redemption and every failed authorization are therefore both
  exact-once;
- explicit revocation and expiry purging destroy retained authority;
- diagnostics expose counts and error categories, never targets, field names,
  keys, or values.

The later service and UniFFI layers must own one registry per bounded authority
domain. They may serialize the opaque token but must never serialize a reviewed
or authorized plan as a substitute.

## Evidence

`tablerock-core/tests/mutation.rs` proves one successful redemption, replay
rejection, consumption after scope failure, finite capacity, duplicate-token
rejection, expiry purging, and explicit revocation. Existing mutation tests
continue proving plan bounds, redaction, and revision/expiry validation.

This contract is derived from TableRock's fixed safety and shared-client
requirements. It uses no external product implementation or protected
expression.
