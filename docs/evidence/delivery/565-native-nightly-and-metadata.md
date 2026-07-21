# 565 — Native Nightly and artifact metadata

Date: 2026-07-21

## Decision

Native checkpoint and Nightly automation are separate trunk-only workflows.
Checkpoint runs on every relevant `main` push. Nightly runs on schedule or
manual dispatch with the committed `Nightly` Xcode test plan, exact unsigned
Release archive, accessibility runtime gate, Time Profiler capture, bounded RSS
sampling, leak scan, normalized app/linkage/signing metadata, and SHA-256 files.

The shared metadata recorder consumes the canonical `.xcarchive` and universal
static XCFramework. It fails if the application retains an absolute `target/`
path or development `libtablerock_ffi.dylib` linkage.

## Hosted proof

Checkpoint run:
[29829989957](https://github.com/tailrocks/tablerock/actions/runs/29829989957),
commit `ed38df4d7e69f0356951a5bb8ca58374b0069073`.

- 8 feature + 13 bridge + 8 application-model + 6 UI tests = 35 tests,
  0 failures.
- Profile creation operates the real Add control; multi-window automation
  observes two independent workbench windows.
- Universal XCFramework, Release archive, development app, strict signing,
  architecture/linkage checks, and metadata recording passed.
- Artifact `native-checkpoint-ed38df4d7e69f0356951a5bb8ca58374b0069073`:
  ID `8495935411`, 157,454,029 bytes, SHA-256
  `5e900fc0f2980f67f96743728c4ee562e98d605fa0ccf7bba9160302e18470e4`.

Nightly run:
[29831461819](https://github.com/tailrocks/tablerock/actions/runs/29831461819),
commit `21116423fe33b4456ea56f989050706110f3090a`.

- Host: macOS 26.4, Xcode 26.6, Swift 6.3.3, Rust 1.97.1; the workflow
  refreshes Homebrew and verifies current ripgrep 15.2.0 before running the
  repository scripts.
- The same 35-test plan passed; all six UI workflows completed in 53.784
  seconds.
- Canonical Release archive and accessibility structural/runtime gate passed.
- 10,000-row fixture build: maximum reported 0.037708 seconds.
- Automated scroll: 2.009304 seconds; maximum RSS: 135,776 KiB.
- Time Profiler trace: 12,992,512 bytes.
- Leak scan: 0 leaks, 0 leaked bytes.
- Artifact `native-nightly-21116423fe33b4456ea56f989050706110f3090a`:
  ID `8496178522`, 129,161,802 bytes, SHA-256
  `05c9d481ac0e469580cb614abd676ffe6d67036ba00e43426a485cb30089ce43`.

The artifacts retain `.xcresult`, Xcode/archive logs, `.xcarchive`, Time
Profiler trace and TOC, RSS samples, leak report, crash reports, application
metadata, architecture/linkage reports, codesign facts, and file checksums.

## Remaining boundary

This completes the scheduled unsigned structural/performance foundation of
checkpoints 14–16. It does not claim the remaining live PostgreSQL app-process
case, all-three-engine live bridge execution on macOS, IME/marked-text and full
accessibility matrices, Allocations template evidence, clean-machine lifecycle,
or Developer ID/notarization/stapling proof.

## Provenance

External concepts: Xcode test plans and `.xcresult`, Instruments Time Profiler,
GitHub Actions scheduled workflows and retained artifacts.

Implementation source: TableRock-owned tests, fixtures, scripts, workflows,
XcodeGen specification, and application.

TablePro influence: none; this checkpoint changes testing and delivery
infrastructure, not product workflow or visual expression.

Copied code, tests, assets, strings, identifiers, colors, geometry, layout
measurements, or key bindings: none.
