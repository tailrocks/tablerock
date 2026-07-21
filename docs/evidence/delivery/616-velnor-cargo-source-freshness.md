# Velnor Cargo source freshness barrier

Date: 2026-07-22

## Failure class

CI run `29875329220` checked out exact commit `13fa79e` but Velnor preserved a
workspace `target/` while pinning source mtimes to commit time. Existing build
outputs were newer than the changed `tablerock-core` sources, so Cargo skipped
the core rebuild and clippy linked a stale rlib without the new saved-filter
exports. The exact source files and commit were present; this was unsafe
filesystem-time reuse, not a Rust dependency error.

## Correction

Both Velnor compilation jobs refresh Rust source and manifest mtimes after
checkout and before Cargo runs. This forces Cargo to recompute changed crate
fingerprints while sccache retains safe content-addressed compiler reuse.
GitHub-hosted lanes keep their normal clean-workspace behavior.

## Verification

The failed job log records exact checkout `13fa79e`, then no
`Checking tablerock-core` line before unresolved exports in `tablerock-tui`.
Local clean compilation, 497 relevant nextest cases, and clippy were already
green from the same source. Exact-main hosted run `29875658673` then passed
format, clippy, check, all container-free suites, the full three-engine
real-server matrix, UniFFI conformance, import/export, PostgreSQL client-tool
coverage, performance budgets, and the true 1-MiB-tmpfs ENOSPC gate. Its log
shows the freshness step before compilation; no stale export failure recurred.

## Provenance

No external product reference influenced this CI correction. It derives from
the exact hosted checkout/build log and Cargo's local source/output boundary.
