# Native Release demo removal

Date: 2026-08-13

Checkpoint: P6c — eliminate remaining fixture behavior from production

## Decision

Release no longer compiles a scripted backend choice or the hard-coded
edit-safety probe. The probe exposed a production UniFFI method that staged a
DELETE against `public.users` with locator `1`; Swift surfaced it as a query
toolbar action and Change Review sheet. That was demo behavior, not a real
operator workflow, so compile-time UI hiding was insufficient.

The exported Rust method, generated bindings, Feature protocol methods, live
adapter methods, presentation state/actions/sheet, scripted implementation,
and live Swift test were removed together. The Rust-only
`insert_reviewed_probe` seam remains outside UniFFI for conformance testing of
consume-once/expiry/apply authority. Real DDL, table-operation, CSV-import, and
PostgreSQL role-change review paths remain production behavior.

`AppConfiguration.Backend.scripted` and its errors now exist only when
`TABLEROCK_DEVELOPMENT_SUPPORT` is compiled. Release has one backend case:
`live`. Debug and Test Release retain deterministic scripted support for tests.

Generic production names were also corrected: Change Review's warning is a
`safetyNote`, and grid performance automation owns a
`performanceScrollTask`. Neither surface claims fixture ownership.

## Structural guard

`scripts/verify-native-source-ownership.sh` now rejects reintroduction of the
retired exported demo API in Rust, generated UniFFI bindings, or production
Swift sources. Existing ownership checks still require fixture routes and
`ScriptedWorkbenchBackend` implementations to remain in compile-time-guarded
DevelopmentSupport files.

## Evidence

- Regenerated UniFFI Swift/header output contains no `stage_probe_review` or
  `stageProbeReview` symbol.
- `cargo test -p tablerock-ffi`: 52 passed, 5 ignored, 0 failed across 9 suites.
- SwiftPM Debug: 56 XCTest cases, 3 expected live-server skips, 0 failures;
  23 Swift Testing cases passed.
- SwiftPM Release: 53 XCTest cases, 3 expected live-server skips, 0 failures;
  23 Swift Testing cases passed.
- `scripts/build-native-app.sh`, `scripts/verify-native-source-ownership.sh`,
  and `scripts/verify-native-query-tabs.sh` passed.
- Xcode Release archive passed for arm64 and x86_64. String and exported-symbol
  scans covered the app executable plus Presentation, Bridge, and Feature
  frameworks and found no fixture, scripted backend, or retired probe symbols.
- `git diff --check` passed.

## Clean-room provenance

This checkpoint removes test/demo residue and preserves TableRock's real review
workflows. TablePro's public user-visible native application informed the
confirmed production direction only. No TablePro source, tests, comments,
bundle internals, branding, assets, copy, or proprietary fixtures were used.
