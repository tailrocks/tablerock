# 564 — Canonical native Xcode checkpoint

Date: 2026-07-21

## Decision

The generated Xcode project, shared `TableRock` scheme, and committed
`Checkpoint`, `Nightly`, and `Release` test plans are now the canonical native
application surface. The every-push checkpoint consumes the universal static
XCFramework, executes the deterministic model/bridge/application suites and
five user-operable UI workflows, then archives the same Release application
shape used by the Developer ID workflow.

The cancellation workflow launches a real app process with the injected
`slow-until-cancelled` backend, clicks the enabled Cancel control, and observes
the model's semantic cancellation outcome through a stable accessibility
value. Swift task cancellation alone is not treated as Rust cancellation.

## Hosted proof

Run: [29827520875](https://github.com/tailrocks/tablerock/actions/runs/29827520875),
commit `9763873286678bfb6fc91ec1a68d9d77286512f0`.

- Host: macOS 26.4, Xcode 26.6 (17F113), Swift 6.3.3, Rust 1.97.1.
- SwiftPM regression suite: 21 tests, 0 failures.
- Canonical Checkpoint plan: 8 feature + 13 bridge + 8 application-model +
  5 UI tests = 34 tests, 0 failures.
- The UI suite completed all five workflows in 47.956 seconds, including
  slow query cancellation through the application boundary.
- Generated project and UniFFI binding regeneration produced no diff.
- Universal static XCFramework build passed for arm64 and x86_64.
- Canonical Release archive succeeded. Its application executable is arm64 +
  x86_64, strict ad-hoc codesign verification passed, bundle identifier is
  `app.tablerock.TableRock`, and no development dylib or absolute `target/`
  linkage exists.
- Artifact `native-checkpoint-9763873286678bfb6fc91ec1a68d9d77286512f0`:
  ID `8494544351`, 200,406,687 bytes, SHA-256
  `6eb9e0f68feb4d080c7ac8fe4c8d4d913602d1f9850f2799552a97bbd84811f8`.
  It contains the `.xcresult`, Xcode logs, canonical `.xcarchive`, development
  app, XCFramework, generated bindings, project, and test plans.

This completes checkpoint 12's named model/deallocation proof, checkpoint 13,
and the deterministic portion of checkpoint 14. It establishes checkpoint
15's exact unsigned shipping shape and checkpoint 16's checkpoint artifacts.
Nightly live-server/UI/performance coverage and clean-machine signed release
proof remain separate requirements.

## Distribution boundary

This evidence makes no Developer ID, notarization, stapling, Gatekeeper, clean
machine, update, or uninstall claim. Those gates require operator-provisioned
Apple signing and notary credentials and a suitable clean-machine environment.

## Provenance

External concepts: Xcode test plans, XCTest UI automation, XCFramework archive
linkage, GitHub Actions artifact retention.

Implementation source: TableRock-owned application contracts, tests, XcodeGen
specification, scripts, and workflow.

TablePro influence: none; this checkpoint changes testing and delivery
infrastructure, not product workflow or visual expression.

Copied code, tests, assets, strings, identifiers, colors, geometry, layout
measurements, or key bindings: none.
