# Plan 020 — live vertical-slice conformance

Date: 2026-07-19

## Correction

The Swift `BehaviorProof` contained a reviewed-operation path, but the native
behavior verifier never invoked it. Therefore earlier query/catalog/cancel
evidence did not prove the complete vertical slice. The verifier now creates a
bounded PostgreSQL fixture row and runs the same UniFFI review-token API used by
the app through stage, consume-once authorization, and apply. Exact container
cleanup also runs on every script exit.

## Live matrix

`./scripts/verify-native-behavior.sh` built the strict Swift 6 bridge/app and
passed:

| Engine | Query/page decode | Typed catalog | Cancellation | Reviewed apply |
|---|---:|---:|---:|---:|
| PostgreSQL 18.4 | pass | pass, 68 rows | server-confirmed, 0.173 s | committed, 1 applied |
| ClickHouse 25.8 | pass | pass, 150 rows | engine contract suite | not applicable to the PostgreSQL probe |
| Redis 8.0 | pass | pass | engine contract suite | not applicable to the PostgreSQL probe |

The executable path is the generated synchronous UniFFI facade and the same
hostile-bounds `PageV1` Swift decoder consumed by `BridgeClient`. It proves
connect → typed catalog → execute → event pump → immutable page decode, plus
operation-ID cancellation and the applicable reviewed mutation. No per-cell
bridge entry occurs.

## Swift ownership audit

Repository inspection of `native/Sources` finds no database client import, SQL
parser, statement classifier, mutation-plan constructor, or safety/redaction
policy in Swift. Swift passes editor text as an opaque statement and renders
typed Rust outcomes. The sample `SELECT 1;` editor content is presentation
fixture text, not parsing or policy. Catalog intent and reviewed mutation
construction remain Rust-owned.

## Provenance

TablePro was used only as a broad conceptual reference for the connect,
catalog, query, result, cancel, and reviewed-operation workflow. No source,
tests, text, screenshots, layouts, measurements, colors, assets, or key
bindings were copied or translated.
